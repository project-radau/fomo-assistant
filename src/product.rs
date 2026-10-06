pub struct Product {
    name: String,
    value: f64
}

impl Product {
    pub fn new(name: String, value: f64) -> Self {
        Self {
            name,
            value
        }
    }

    pub fn render(&self) {
        println!("Product Name: {}", self.name);
        println!("Current Price: {}", self.value);
    }
}