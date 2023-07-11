mod parser;

fn main() {
    // Example usage
    let input = Span::new("(+ 1 (* 2 3))");

    match parse_expr::<()>(input) {
        Ok((_, expr)) => println!("Parsed expression: {:?}", expr),
        Err(e) => println!("Error parsing input: {:?}", e),
    }
}
