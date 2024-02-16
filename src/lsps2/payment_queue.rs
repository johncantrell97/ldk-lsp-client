use crate::lsps2::msgs::OpeningFeeParams;
use crate::lsps2::utils::compute_opening_fee;
use crate::prelude::{HashMap, String, ToString, Vec};
use alloc::collections::VecDeque;
use lightning::ln::channelmanager::InterceptId;
use lightning::ln::PaymentHash;

/// Holds payments with the corresponding HTLCs until it is possible to pay the fee.
/// When the fee is succesfully paid with a forwarded payment, the queue should be consumed and the
/// remaining payments forwarded.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct PaymentQueue {
	opening_fee_params: OpeningFeeParams,
	payment_size_msat: Option<u64>,
	opening_fee_msat: Option<u64>,

	payments: HashMap<PaymentHash, Vec<InterceptedHTLC>>,
	sufficient: VecDeque<PaymentHash>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct OpenChannelParams {
	pub(crate) opening_fee_msat: u64,
	pub(crate) amt_to_forward_msat: u64,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct InterceptedHTLC {
	pub(crate) intercept_id: InterceptId,
	pub(crate) expected_outbound_amount_msat: u64,
	pub(crate) payment_hash: PaymentHash,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct FeePayment {
	pub(crate) htlcs: Vec<InterceptedHTLC>,
	pub(crate) opening_fee_msat: u64,
}

#[derive(Debug)]
pub(crate) struct HTLCError(pub(crate) String);

impl PaymentQueue {
	pub(crate) fn new(
		opening_fee_params: OpeningFeeParams, payment_size_msat: Option<u64>,
	) -> PaymentQueue {
		PaymentQueue {
			opening_fee_params,
			payment_size_msat,
			opening_fee_msat: None,

			payments: HashMap::new(),
			sufficient: VecDeque::new(),
		}
	}

	pub(crate) fn add_htlc(
		&mut self, htlc: InterceptedHTLC,
	) -> Result<Option<OpenChannelParams>, HTLCError> {
		let payment_hash = htlc.payment_hash;
		let htlcs = self.payments.entry(payment_hash).or_insert(vec![]);
		htlcs.push(htlc);

		let total_expected_outbound_amount_msat =
			htlcs.iter().map(|htlc| htlc.expected_outbound_amount_msat).sum();

		let (expected_payment_size_msat, mpp_mode) =
			if let Some(payment_size_msat) = self.payment_size_msat {
				(payment_size_msat, true)
			} else {
				debug_assert_eq!(htlcs.len(), 1);
				if htlcs.len() != 1 {
					return Err(HTLCError(format!(
						"Paying via multiple HTLCs is disallowed in \"no-MPP+var-invoice\" mode."
					)));
				}
				(total_expected_outbound_amount_msat, false)
			};

		if expected_payment_size_msat < self.opening_fee_params.min_payment_size_msat
			|| expected_payment_size_msat > self.opening_fee_params.max_payment_size_msat
		{
			return Err(HTLCError(
					format!("Payment size violates our limits: expected_payment_size_msat = {}, min_payment_size_msat = {}, max_payment_size_msat = {}",
							expected_payment_size_msat,
							self.opening_fee_params.min_payment_size_msat,
							self.opening_fee_params.max_payment_size_msat
					)));
		}

		let opening_fee_msat = if let Some(fee) = self.opening_fee_msat {
			fee
		} else {
			compute_opening_fee(
				expected_payment_size_msat,
				self.opening_fee_params.min_fee_msat,
				self.opening_fee_params.proportional.into(),
			).ok_or(HTLCError(
				format!("Could not compute valid opening fee with min_fee_msat = {}, proportional = {}, and expected_payment_size_msat = {}",
					self.opening_fee_params.min_fee_msat,
					self.opening_fee_params.proportional,
					expected_payment_size_msat
				)
			))?
		};
		let amt_to_forward_msat = expected_payment_size_msat.saturating_sub(opening_fee_msat);

		// Sufficient HTLCs are intercepted.
		if total_expected_outbound_amount_msat >= expected_payment_size_msat
			&& amt_to_forward_msat > 0
		{
			if !self.sufficient.contains(&payment_hash) {
				self.sufficient.push_back(payment_hash);
			}
			if self.opening_fee_msat.is_none() {
				self.opening_fee_msat = Some(opening_fee_msat);
				Ok(Some(OpenChannelParams { opening_fee_msat, amt_to_forward_msat }))
			} else {
				Ok(None)
			}
		} else {
			if mpp_mode {
				Ok(None)
			} else {
				Err(HTLCError("Intercepted HTLC is too small to pay opening fee".to_string()))
			}
		}
	}

	pub(crate) fn pop_fee_payment(&mut self) -> Option<FeePayment> {
		if let Some(opening_fee_msat) = self.opening_fee_msat {
			let hash = self.sufficient.pop_front();
			let htlcs = hash.and_then(|hash| self.payments.remove(&hash));
			htlcs.map(|htlcs| FeePayment { htlcs, opening_fee_msat })
		} else {
			None
		}
	}

	pub(crate) fn clear(&mut self) -> Vec<InterceptedHTLC> {
		self.sufficient.clear();
		self.payments.drain().map(|(_k, v)| v).flatten().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::{TimeZone, Utc};

	#[test]
	fn test_payment_queue() {
		let mut payment_queue = PaymentQueue::new(
			OpeningFeeParams {
				min_fee_msat: 10_000_000,
				proportional: 10_000,
				valid_until: Utc.timestamp_opt(3000, 0).unwrap(),
				min_lifetime: 4032,
				max_client_to_self_delay: 2016,
				min_payment_size_msat: 10_000_000,
				max_payment_size_msat: 1_000_000_000,
				promise: "ignore".to_string(),
			},
			Some(500_000_000),
		);

		assert!(payment_queue
			.add_htlc(InterceptedHTLC {
				intercept_id: InterceptId([0; 32]),
				expected_outbound_amount_msat: 200_000_000,
				payment_hash: PaymentHash([100; 32]),
			})
			.unwrap()
			.is_none());
		assert!(payment_queue.pop_fee_payment().is_none());

		assert!(payment_queue
			.add_htlc(InterceptedHTLC {
				intercept_id: InterceptId([1; 32]),
				expected_outbound_amount_msat: 300_000_000,
				payment_hash: PaymentHash([101; 32]),
			})
			.unwrap()
			.is_none());
		assert!(payment_queue.pop_fee_payment().is_none());

		assert_eq!(
			payment_queue
				.add_htlc(InterceptedHTLC {
					intercept_id: InterceptId([2; 32]),
					expected_outbound_amount_msat: 300_000_000,
					payment_hash: PaymentHash([100; 32]),
				})
				.unwrap(),
			Some(OpenChannelParams {
				opening_fee_msat: 10_000_000,
				amt_to_forward_msat: 490_000_000,
			})
		);
		assert_eq!(
			payment_queue.pop_fee_payment(),
			Some(FeePayment {
				opening_fee_msat: 10_000_000,
				htlcs: vec![
					InterceptedHTLC {
						intercept_id: InterceptId([0; 32]),
						expected_outbound_amount_msat: 200_000_000,
						payment_hash: PaymentHash([100; 32]),
					},
					InterceptedHTLC {
						intercept_id: InterceptId([2; 32]),
						expected_outbound_amount_msat: 300_000_000,
						payment_hash: PaymentHash([100; 32]),
					},
				],
			})
		);
		assert_eq!(
			payment_queue.clear(),
			vec![InterceptedHTLC {
				intercept_id: InterceptId([1; 32]),
				expected_outbound_amount_msat: 300_000_000,
				payment_hash: PaymentHash([101; 32]),
			}]
		);
	}
}
