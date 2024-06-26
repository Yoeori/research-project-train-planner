#[macro_use] mod alg_macros;
pub mod td_simple_vec;
pub mod td_simple_btree;
pub mod csa_btree;
pub mod csa_vec;
pub mod raptor;
pub mod raptor_btree;

use td_simple_btree::TDSimpleBTree;
use td_simple_vec::TDSimpleVec;
use csa_btree::CSABTree;
use csa_vec::CSAVec;
use raptor::Raptor;
use raptor_btree::RaptorBTree;

use crate::{benchable::{BenchableLive, Benchable}, types::Timetable};

/// Retreives a list of initializers for benchables
#[allow(dead_code)]
pub fn algorithms() -> &'static [for<'a> fn(&'a Timetable) -> Box<dyn Benchable<'a> + 'a>] {
    &[
        |t| Box::new(CSABTree::new(t)) as Box<dyn Benchable>,
        |t| Box::new(CSAVec::new(t)) as Box<dyn Benchable>,
        |t| Box::new(TDSimpleVec::new(t)) as Box<dyn Benchable>,
        |t| Box::new(TDSimpleBTree::new(t)) as Box<dyn Benchable>,
        |t| Box::new(Raptor::new(t)) as Box<dyn Benchable>,
        |t| Box::new(RaptorBTree::new(t)) as Box<dyn Benchable>
    ]
}

#[allow(dead_code)]
pub fn algorithms_live() -> &'static [for<'a> fn(&'a Timetable) -> Box<dyn BenchableLive<'a> + 'a>] {
    &[
        |t| Box::new(CSABTree::new(t)) as Box<dyn BenchableLive>,
        |t| Box::new(TDSimpleBTree::new(t)) as Box<dyn BenchableLive>,
        |t| Box::new(RaptorBTree::new(t)) as Box<dyn BenchableLive>
    ]
}