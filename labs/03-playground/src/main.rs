// 1. Defining a trait (just like 'type HealthCheck interface' in Go)
trait HealthCheck {
    fn is_healthy(&self) -> bool;
    fn status_report(&self) -> String;
}

struct PostgresNode {
    host: String,
    active_queries: u32,
}

// 2. Explicitly implementing the trait for our struct
impl HealthCheck for PostgresNode {
    fn is_healthy(&self) -> bool {
        self.active_queries < 1000
    }

    fn status_report(&self) -> String {
        format!("DB {} load: {} queries", self.host, self.active_queries)
    }
}

impl PostgresNode {
    fn is_healthy(&self) -> bool {
        self.active_queries < 100
    }
}

fn main() {
    let db = PostgresNode {
        host: String::from("10.0.0.5"),
        active_queries: 42,
    };

    println!("Healthy: {} | {}", db.is_healthy(), db.status_report());
}