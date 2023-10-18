use bitcoin::secp256k1::PublicKey;
use lightning::sign::EntropySource;
use std::{fmt::Write, ops::Deref};

use crate::transport::msgs::RequestId;

pub(crate) fn generate_request_id<ES: Deref>(entropy_source: &ES) -> RequestId
where
	ES::Target: EntropySource,
{
	let bytes = entropy_source.get_secure_random_bytes();
	RequestId(hex_str(&bytes[0..16]))
}

#[inline]
pub fn hex_str(value: &[u8]) -> String {
	let mut res = String::with_capacity(2 * value.len());
	for v in value {
		write!(&mut res, "{:02x}", v).expect("Unable to write");
	}
	res
}

pub fn to_vec(hex: &str) -> Option<Vec<u8>> {
	let mut out = Vec::with_capacity(hex.len() / 2);

	let mut b = 0;
	for (idx, c) in hex.as_bytes().iter().enumerate() {
		b <<= 4;
		match *c {
			b'A'..=b'F' => b |= c - b'A' + 10,
			b'a'..=b'f' => b |= c - b'a' + 10,
			b'0'..=b'9' => b |= c - b'0',
			_ => return None,
		}
		if (idx & 1) == 1 {
			out.push(b);
			b = 0;
		}
	}

	Some(out)
}

pub fn to_compressed_pubkey(hex: &str) -> Option<PublicKey> {
	if hex.len() != 33 * 2 {
		return None;
	}
	let data = match to_vec(&hex[0..33 * 2]) {
		Some(bytes) => bytes,
		None => return None,
	};
	match PublicKey::from_slice(&data) {
		Ok(pk) => Some(pk),
		Err(_) => None,
	}
}

pub fn parse_pubkey(pubkey_str: &str) -> Result<PublicKey, std::io::Error> {
	let pubkey = to_compressed_pubkey(pubkey_str);
	if pubkey.is_none() {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Other,
			"ERROR: unable to parse given pubkey for node",
		));
	}

	Ok(pubkey.unwrap())
}

pub fn compute_opening_fee(
	payment_size_msat: u64, opening_fee_min_fee_msat: u64, opening_fee_proportional: u64,
) -> Option<u64> {
	let t1 = payment_size_msat.checked_mul(opening_fee_proportional)?;
	let t2 = t1.checked_add(999999)?;
	let t3 = t2.checked_div(1000000)?;
	let t4 = std::cmp::max(t3, opening_fee_min_fee_msat);
	Some(t4)
}
