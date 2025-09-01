fn main() {
    let value = serde_json::to_string(&"8688888888888".to_owned());
    println!("{}", value.unwrap())
}
