use super::{Node, NodeMeta, ParNode, Param};

#[derive(Clone, Debug)]
pub struct Chain2<N, M> {
    first: N,
    second: M,
    out: f32,
}

impl<N: Node, M: Node> Chain2<N, M> {
    fn new(first: N, second: M) -> Self {
        Chain2 {
            first,
            second,
            out: 0.0,
        }
    }
}

impl<N: Node + Clone, M: Node + Clone> Node for Chain2<N, M> {
    fn feed(&mut self, samples: &[f32]) {
        self.first.feed(samples);
        self.second.feed(&[self.first.read()]);
        self.out = self.second.read();
    }

    fn read(&self) -> f32 {
        self.out
    }

    fn set_param(&mut self, value: &[Param]) {
        let params_first = self.first.meta().params.len();
        if params_first > 0 {
            self.first.set_param(&value[0..params_first]);
        }
        if params_first < value.len() {
            self.second.set_param(&value[params_first..value.len()]);
        }
    }

    fn get_param(&self) -> Vec<Param> {
        let mut param = self.first.get_param();
        param.extend(self.second.get_param().into_iter());

        param
    }

    fn meta(&self) -> NodeMeta {
        NodeMeta::new(
            self.first.meta().inputs.iter(),
            self.first
                .meta()
                .params
                .iter()
                .map(|(s, p)| (s as &str, p.clone()))
                .chain(
                    self.second
                        .meta()
                        .params
                        .iter()
                        .map(|(s, p)| (s as &str, p.clone())),
                ),
        )
    }
}

pub fn chain2<N: Node, M: Node>(first: N, second: M) -> Chain2<N, M> {
    Chain2::new(first, second)
}

#[derive(Clone, Debug)]
pub struct FeedbackMany<F, B, O> {
    forward: F,
    backward: B,
    output: O,
    out: f32,
}

impl<F: ParNode + Clone, B: ParNode + Clone, O: Node + Clone> FeedbackMany<F, B, O> {
    fn new(forward: F, backward: B, output: O) -> Self {
        FeedbackMany {
            forward,
            backward,
            output,
            out: 0.0,
        }
    }
}

impl<F: ParNode + Clone + Send, B: ParNode + Clone + Send, O: Node + Clone + Send> Node
    for FeedbackMany<F, B, O>
{
    fn feed(&mut self, samples: &[f32]) {
        let mut forward_in = Vec::from(samples);
        let mut forward_out = vec![0.0; samples.len()];
        let mut backward_out = vec![0.0; samples.len()];

        self.backward.read(&mut backward_out);
        for i in 0..samples.len() {
            forward_in[i] += backward_out[i];
        }

        self.forward.feed_split(&forward_in);
        self.forward.read(&mut forward_out);

        self.output.feed(&forward_out);
        self.out = self.output.read();

        self.backward.feed_par(&forward_out);
    }

    fn read(&self) -> f32 {
        self.out
    }

    // TODO :<
    fn set_param(&mut self, _value: &[Param]) {
        unreachable!()
    }

    // TODO :<
    fn get_param(&self) -> Vec<Param> {
        unreachable!()
    }

    // TODO :<
    fn meta(&self) -> NodeMeta {
        self.forward.meta_as_split()
    }
}

pub fn feedback_many<F: ParNode + Clone, B: ParNode + Clone, O: Node + Clone>(
    forward: F,
    backward: B,
    output: O,
) -> FeedbackMany<F, B, O> {
    FeedbackMany::new(forward, backward, output)
}
