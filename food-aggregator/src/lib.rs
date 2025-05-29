mod sources;

pub struct AggregateCtx {}

#[derive(Debug)]
enum AggregateStatus {
    Continue,
    Finished,
    PendingFor(u64),
}

pub trait Aggregator {
    async fn aggregate(&self) -> AggregateStatus;
}
