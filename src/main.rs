// Import our units library
use runits::units::{ConversionError, Quantity, Unit};

fn main() {
    println!("=== RUnits Demo - Testing Your Implementation ===\n");

    println!("1. Creating quantities:");
    let distance = Quantity::meters(100.0);
    println!("Great! You have {}", distance.to_string());

    println!("\n2. Successful conversions:");
    // Examples: feet to meters, miles to kilometers, minutes to seconds
    // Handle the Result using match or unwrap()
    let target_unit = Unit::meter();
    let feet = Quantity::new(10.0, Unit::foot());
    print_conversion_result(&feet, feet.convert_to(&target_unit));

    println!("\n3. Error handling:");
    // Example: try to convert meters to seconds
    // Use match to handle both Ok and Err cases properly
    let target_unit = Unit::second();
    let weight = Quantity::kilograms(90.0);
    print_conversion_result(&weight, weight.convert_to(&target_unit));

    println!("\n4. Complex example:");
    // Example: "I ran 5 miles, how many kilometers is that?"
    // Print both the original quantity and the converted result
    let target_unit = Unit::kilometer();
    let distance = Quantity::new(5.0, Unit::mile());
    println!("You have: {}", distance.to_string());
    println!("You want: {}", target_unit.name);
    print_conversion_result(&distance, distance.convert_to(&target_unit));
}

// This function should take a conversion result and print it nicely
// Handle both success and error cases
fn print_conversion_result(original: &Quantity, result: Result<Quantity, ConversionError>) {
    // Your code here:
    // Use match to handle both Ok(converted_quantity) and Err(error)
    // For Ok: print "X unit -> Y target_unit"
    // For Err: print "Error: <error message>"
    match result {
        Ok(converted) => println!("{} -> {}", original.to_string(), converted.to_string()),
        Err(error) => println!("Error: {}", error),
    }
}
