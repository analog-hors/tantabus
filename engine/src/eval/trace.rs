use super::EvalTerms;

pub trait TraceTarget {
    fn trace(&mut self, term: impl FnMut(&mut EvalTerms));
}

impl TraceTarget for EvalTerms {
    fn trace(&mut self, mut term: impl FnMut(&mut EvalTerms)) {
        term(self);
    }
}

impl TraceTarget for () {
    fn trace(&mut self, _: impl FnMut(&mut EvalTerms)) {
    }
}
