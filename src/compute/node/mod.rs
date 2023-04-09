use std::{borrow::Cow, fmt::Debug};

use dyn_clone::DynClone;

pub mod basic;
pub mod compose;
pub mod filter;

pub struct NodeMeta {
    pub inputs: Vec<String>,
    pub params: Vec<(String, ParamSignature)>,
}

impl NodeMeta {
    pub fn new<T1, T2, I1, I2>(inputs: I1, params: I2) -> Self
    where
        T1: Into<String>,
        T2: Into<String>,
        I1: IntoIterator<Item = T1>,
        I2: IntoIterator<Item = (T2, ParamSignature)>,
    {
        NodeMeta {
            inputs: inputs.into_iter().map(Into::into).collect(),
            params: params.into_iter().map(|(s, pt)| (s.into(), pt)).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamValueType {
    F,
    FDyn,
    U,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamSignatureEntry {
    pub ty: ParamValueType,
    pub name: Cow<'static, str>,
}

impl Eq for ParamSignatureEntry {}

#[macro_export]
macro_rules! param {
    ( $( $name:tt $t:ident ),* ) => {
        {
            let mut v = ::std::vec::Vec::new();
            $(
                v.push($crate::compute::node::ParamSignatureEntry {
                    ty: $crate::compute::node::ParamValueType::$t,
                    name: ::std::borrow::Cow::Borrowed(stringify!($name)),
                });
            )*

            $crate::compute::node::ParamSignature(v)
        }
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamSignature(pub Vec<ParamSignatureEntry>);

#[derive(Clone, Debug)]
pub enum ParamValue {
    F(f32),
    FDyn(Vec<f32>),
    U(u32),
}

impl ParamValue {
    fn as_f(&self) -> f32 {
        match self {
            ParamValue::F(f) => *f,
            _ => panic!(),
        }
    }

    fn as_fdyn(&self) -> &[f32] {
        match self {
            ParamValue::FDyn(f) => f.as_slice(),
            _ => panic!(),
        }
    }

    fn as_u(&self) -> u32 {
        match self {
            ParamValue::U(u) => *u,
            _ => panic!(),
        }
    }
}

impl From<f32> for ParamValue {
    fn from(value: f32) -> Self {
        ParamValue::F(value)
    }
}

impl From<u32> for ParamValue {
    fn from(value: u32) -> Self {
        ParamValue::U(value)
    }
}

impl From<i32> for ParamValue {
    fn from(value: i32) -> Self {
        if value >= 0 {
            ParamValue::U(value as u32)
        } else {
            panic!()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Param(pub Vec<ParamValue>);

pub trait IntoParams {
    fn into_param(self) -> Param;
}

impl<T: Into<ParamValue>> IntoParams for T {
    fn into_param(self) -> Param {
        Param(vec![self.into()])
    }
}

macro_rules! param_into_iter {
    ( $( $t:ident $n:tt ),* ) => {
        impl<$( $t: Into<ParamValue>, )*> IntoParams for ($( $t, )*) {
            fn into_param(self) -> Param {
                Param(vec![
                    $(
                        self.$n.into(),
                    )*
                ])
            }
        }
    };
}

param_into_iter!(N0 0);
param_into_iter!(N0 0, N1 1);
param_into_iter!(N0 0, N1 1, N2 2);
param_into_iter!(N0 0, N1 1, N2 2, N3 3);
param_into_iter!(N0 0, N1 1, N2 2, N3 3, N4 4);

pub trait WithParam {
    fn with_param(self, param: impl IntoParams) -> Self;
}

impl<N: Node> WithParam for N {
    fn with_param(mut self, param: impl IntoParams) -> Self {
        self.set_param(&[param.into_param()]);
        self
    }
}

pub trait Node: DynClone + Debug + Send {
    fn feed(&mut self, samples: &[f32]);
    fn read(&self) -> f32;
    fn set_param(&mut self, value: &[Param]);
    fn get_param(&self) -> Vec<Param>;
    fn meta(&self) -> NodeMeta;
}

pub trait ParNode: DynClone + Debug {
    fn feed_par(&mut self, samples: &[f32]);
    fn feed_split(&mut self, samples: &[f32]);
    fn read(&self, out: &mut [f32]);
    fn meta_as_split(&self) -> NodeMeta;
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

    fn meta_as_split(&self) -> NodeMeta {
        NodeMeta::new(["sig 1"], self.meta().params)
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

            fn meta_as_split(&self) -> NodeMeta {
                NodeMeta::new([
                    $(
                        format!("sig {}", $i),
                    )*],
                    std::iter::empty() $(
                        .chain(self.$i.meta().params.iter().map(|(s, pt)| (s as &str, pt.clone())))
                    )*,
                )
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

pub trait NodeList {
    fn all(&self) -> Vec<(fn() -> Box<dyn Node>, &'static str)>;
}

pub mod all {
    pub use super::basic::*;
    pub use super::compose::*;
    pub use super::filter::*;
}
