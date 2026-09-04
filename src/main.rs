fn main() {
    let info = hwprobe::detect();
    let json = std::env::args().any(|a| a == "--json");
    if json {
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!("{info:#?}");
    }
}
