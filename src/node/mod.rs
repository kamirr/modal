use std::fmt::Debug;

use dyn_clone::DynClone;

pub mod basic;
pub mod compose;
pub mod filter;

pub trait Node: DynClone + Debug {
    fn feed(&mut self, samples: &[f32]);
    fn read(&self) -> f32;
}

pub trait ParNode: DynClone + Debug {
    fn feed_par(&mut self, samples: &[f32]);
    fn feed_split(&mut self, samples: &[f32]);
    fn read(&self, out: &mut [f32]);
}

impl<N: Node> ParNode for N {
    fn feed_par(&mut self, samples: &[f32]) {
        Node::feed(self, samples);
    }
    fn feed_split(&mut self, samples: &[f32]) {
        self.feed_par(samples);
    }

    fn read(&self, out: &mut [f32]) {
        out[0] = Node::read(self);
    }
}

macro_rules! par_node_def {
    ( $( $n:ident $i:tt ),* ) => {
        impl<$( $n: Node + Clone, )*> ParNode for ($( $n, )*) {
            fn feed_par(&mut self, samples: &[f32]) {
                $(
                    self.$i.feed(samples);
                )*
            }
            fn feed_split(&mut self, samples: &[f32]) {
                $(
                    self.$i.feed(&[samples[$i]]);
                )*
            }

            fn read(&self, out: &mut [f32]) {
                $(
                    out[$i] = self.$i.read();
                )*
            }
        }
    };
}

par_node_def!(N1 0);
par_node_def!(N1 0, N2 1);
par_node_def!(N1 0, N2 1, N3 2);
par_node_def!(N1 0, N2 1, N3 2, N4 3);
par_node_def!(N1 0, N2 1, N3 2, N4 3, N5 4);
par_node_def!(N1 0, N2 1, N3 2, N4 3, N5 4, N6 5);
par_node_def!(N1 0, N2 1, N3 2, N4 3, N5 4, N6 5, N7 6);
par_node_def!(N1 0, N2 1, N3 2, N4 3, N5 4, N6 5, N7 6, N8 7);

pub mod all {
    pub use super::basic::*;
    pub use super::compose::*;
    pub use super::filter::*;
}
