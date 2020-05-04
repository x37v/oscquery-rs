//! # OSCQuery
//!
//! A rust implemention of the [OSCQueryProposal](https://github.com/Vidvox/OSCQueryProposal).

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

pub mod node;
pub mod param;
pub mod root;
pub mod service;
pub mod value;
