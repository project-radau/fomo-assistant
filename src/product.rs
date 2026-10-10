use std::fmt;

pub struct Product {
    url: String,
    name: String,
    value: f64
}

impl std::fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Product Name: {}\nCurrent Price: {}", self.name, self.value)
    }
}

impl Product {
    pub fn new(url: String, name: String, value: f64) -> Self {
        Self {
            url,
            name,
            value
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn value_in_cents(&self) -> i64 {
        (self.value * 100.0) as i64
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}