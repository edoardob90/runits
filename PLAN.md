# **RUnits: Detailed Project Plan**

This plan breaks the project into five distinct phases. Each phase builds on the last, has a clear goal, and focuses on a new set of Rust concepts. This version is designed to be approachable for developers with limited Rust experience.

## **General Guidelines for Beginners**

**Development Best Practices:**
- Run `cargo check` frequently to catch compilation errors early
- Use `cargo clippy` to get helpful suggestions on code quality
- Format your code with `cargo fmt` before committing
- Write tests as you go - they help validate your understanding
- Use `dbg!()` macro liberally for debugging during development

**Resource Recommendations:**
- Keep [The Rust Book](https://doc.rust-lang.org/book/) open as reference
- Use [Rust by Example](https://doc.rust-lang.org/stable/rust-by-example/) for practical examples
- Bookmark [docs.rs](https://docs.rs/) for crate documentation

---

### **Phase 1: The "Brain" - Core Data Structures & Logic**

**Goal:** Create the internal logic for representing and converting units *before* worrying about user input.

**Learning Approach:** Start simple, add complexity gradually. Don't worry about perfection - focus on getting something working.

#### **Tasks:**

1. **Define Core Data Structures:**
   ```rust
   // Start with this basic structure
   #[derive(Debug, Clone, PartialEq)]
   pub enum Dimension {
       Length,
       Mass,
       Time,
       // Add more as needed
   }

   #[derive(Debug, Clone)]
   pub struct Unit {
       pub name: String,
       pub conversion_factor: f64,  // relative to base unit (e.g., meter = 1.0, foot = 0.3048)
       pub dimensions: HashMap<Dimension, i8>,
   }

   #[derive(Debug, Clone)]
   pub struct Quantity {
       pub value: f64,
       pub unit: Unit,
   }
   ```

2. **Implement Basic Unit Creation:**
   - Create helper methods for common units (meter, foot, kilogram, etc.)
   - Example: `Unit::meter()`, `Unit::foot()`
   - Focus on length units first - they're easiest to understand

3. **Implement Simple Conversion Logic:**
   ```rust
   impl Quantity {
       pub fn convert_to(&self, target_unit: &Unit) -> Result<Quantity, ConversionError> {
           // Check dimensional compatibility first
           // Then apply conversion factor
       }
   }
   ```

4. **Create Basic Error Handling:**
   - Start with a simple enum: `enum ConversionError { IncompatibleDimensions }`
   - Use `Result<T, E>` for functions that might fail
   - Don't worry about `thiserror` yet - keep it simple

#### **Validation Checklist:**
- [ ] Can create basic units (meter, foot, kilogram, second)
- [ ] Can convert between compatible units (meter ↔ foot)
- [ ] Conversion fails appropriately for incompatible units (meter ↔ kilogram)
- [ ] Basic tests pass
- [ ] `cargo clippy` gives no warnings

#### **Testing Strategy:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_to_foot_conversion() {
        let one_meter = Quantity::new(1.0, Unit::meter());
        let result = one_meter.convert_to(&Unit::foot()).unwrap();
        assert!((result.value - 3.28084).abs() < 0.0001);
    }

    #[test]
    fn test_incompatible_conversion_fails() {
        let one_meter = Quantity::new(1.0, Unit::meter());
        let result = one_meter.convert_to(&Unit::kilogram());
        assert!(result.is_err());
    }
}
```

#### **Rust Concepts You'll Learn:**
- **struct and enum:** The foundation of data modeling in Rust
- **HashMap:** Key-value storage for unit dimensions
- **impl blocks:** Adding methods to your custom types
- **Ownership and Borrowing:** Why you pass `&Unit` instead of `Unit`
- **Result types:** Rust's approach to error handling (no exceptions!)
- **Basic testing:** Using `#[test]` and `assert!` macros

#### **Common Beginner Pitfalls:**
- **Borrowing confusion:** Use `.clone()` liberally at first, optimize later
- **Float comparison:** Use `(a - b).abs() < epsilon` instead of `a == b`
- **String vs &str:** When in doubt, use `String` for owned data, `&str` for references

---

### **Phase 2: The CLI - A Usable (but Limited) Tool**

**Goal:** Make the program runnable from the command line with a simple, hardcoded parser.

**Learning Approach:** Get something working end-to-end before making it sophisticated.

#### **Tasks:**

1. **Set up clap (Command-Line Argument Parsing):**
   ```rust
   use clap::Parser;

   #[derive(Parser)]
   #[command(author, version, about, long_about = None)]
   struct Cli {
       /// The quantity to convert (e.g., "10.5")
       value: f64,
       
       /// The source unit (e.g., "meter")
       from_unit: String,
       
       /// The target unit (e.g., "foot") 
       to_unit: String,
   }
   ```

2. **Create a Stub Parser:**
   ```rust
   // Simple hardcoded mapping - don't overthink this yet!
   fn parse_unit(unit_name: &str) -> Result<Unit, ParseError> {
       match unit_name.to_lowercase().as_str() {
           "meter" | "m" => Ok(Unit::meter()),
           "foot" | "ft" => Ok(Unit::foot()),
           "kilogram" | "kg" => Ok(Unit::kilogram()),
           // Add a few more...
           _ => Err(ParseError::UnknownUnit(unit_name.to_string())),
       }
   }
   ```

3. **Integrate Everything in main():**
   ```rust
   fn main() -> Result<(), Box<dyn std::error::Error>> {
       let cli = Cli::parse();
       
       let from_unit = parse_unit(&cli.from_unit)?;
       let to_unit = parse_unit(&cli.to_unit)?;
       
       let quantity = Quantity::new(cli.value, from_unit);
       let result = quantity.convert_to(&to_unit)?;
       
       println!("{} {} = {} {}", cli.value, cli.from_unit, result.value, cli.to_unit);
       Ok(())
   }
   ```

#### **File Organization:**
```
src/
├── main.rs          # CLI handling and main()
├── lib.rs           # Re-exports for library interface
├── units.rs         # Unit, Dimension, Quantity structs
├── conversion.rs    # Conversion logic
└── parser.rs        # Simple hardcoded parser
```

#### **Validation Checklist:**
- [ ] `cargo run -- 10 meter foot` produces correct output
- [ ] `cargo run -- 1 kilogram meter` shows appropriate error
- [ ] `cargo run -- 5 unknown_unit meter` shows helpful error message
- [ ] All previous tests still pass
- [ ] New integration tests pass

#### **Testing Strategy:**
```rust
// Integration tests in tests/cli_tests.rs
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn test_basic_conversion() {
    let mut cmd = Command::cargo_bin("runits").unwrap();
    cmd.arg("10").arg("meter").arg("foot");
    
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("32.8084"));
}
```

#### **Rust Concepts You'll Learn:**
- **Crates and Modules:** Using external libraries and organizing code
- **Derive Macros:** `#[derive(Parser)]` and similar magic
- **String vs &str:** You'll encounter this a lot with CLI parsing
- **Error Propagation (?):** The `?` operator for clean error handling
- **Box<dyn Error>:** Type erasure for different error types

#### **Common Beginner Pitfalls:**
- **String ownership:** Use `.to_string()` when you need owned strings
- **Error handling:** Start with `unwrap()`, then replace with `?` 
- **Module confusion:** Use `pub` to make items visible between modules

---

### **Phase 3: The Parser - The Great Leap Forward**

**Goal:** Replace the hardcoded parser with one that understands compound units like `kg*m/s^2`.

**Learning Approach:** This is the most challenging phase. Break it into smaller steps and don't hesitate to look up examples online.

#### **Sub-Phase 3a: Simple Expression Parser**
Start with parsing just `"10 meter"` (number + single unit):

```rust
// grammar.pest
quantity = { number ~ " "+ ~ unit_name }
number = @{ "-"? ~ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
unit_name = @{ ASCII_ALPHA+ }
```

#### **Sub-Phase 3b: Multiplication**
Add support for `"10 meter*second"`:

```rust
// Updated grammar.pest
quantity = { number ~ " "+ ~ unit_expression }
unit_expression = { term ~ ("*" ~ term)* }
term = { unit_name }
```

#### **Sub-Phase 3c: Division and Exponents**
Full expressions like `"10 kg*m/s^2"`:

```rust
// Final grammar.pest  
quantity = { number ~ " "+ ~ unit_expression }
unit_expression = { term ~ (("*" | "/") ~ term)* }
term = { unit_name ~ ("^" ~ exponent)? }
exponent = { "-"? ~ ASCII_DIGIT+ }
```

#### **Parser Implementation:**
```rust
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct UnitParser;

pub fn parse_quantity(input: &str) -> Result<Quantity, ParseError> {
    let pairs = UnitParser::parse(Rule::quantity, input)?;
    
    // Walk the parse tree and build your Quantity
    // This is where you'll use a lot of match expressions
    for pair in pairs {
        match pair.as_rule() {
            Rule::number => {
                // Parse the number
            },
            Rule::unit_expression => {
                // Parse and combine units
            },
            _ => unreachable!(),
        }
    }
}
```

#### **Validation Checklist:**
- [ ] Parse simple quantities: `"10 meter"`, `"2.5 foot"`
- [ ] Parse compound units: `"10 meter*second"`, `"5 kg/m^3"`
- [ ] Handle errors gracefully for invalid syntax
- [ ] Dimensional analysis works for compound units
- [ ] Integration with CLI works correctly

#### **Rust Concepts You'll Learn:**
- **Parsing Theory:** Formal grammars and parse trees
- **The pest Crate:** Parser generators and grammar files
- **Advanced Pattern Matching:** Complex `match` expressions with guards
- **Recursion:** Walking parse trees naturally uses recursion  
- **Error Handling:** Converting pest errors to your custom error types

#### **Common Beginner Pitfalls:**
- **Parse tree complexity:** Use `dbg!()` to understand the tree structure
- **Recursion confusion:** Start with simple, non-recursive cases first
- **Grammar debugging:** Use pest's online debugger to test your grammar

---

### **Phase 4: The Database - Powering Up with Data**

**Goal:** Load a comprehensive set of unit definitions so your tool supports hundreds of units.

#### **Tasks:**

1. **Create a Unit Database:**
   ```rust
   use std::collections::HashMap;
   use lazy_static::lazy_static;

   pub struct UnitDatabase {
       units: HashMap<String, Unit>,
   }

   lazy_static! {
       pub static ref UNIT_DB: UnitDatabase = UnitDatabase::new();
   }
   ```

2. **Populate with Common Units:**
   Start by manually adding 50-100 common units:
   ```rust
   impl UnitDatabase {
       fn new() -> Self {
           let mut units = HashMap::new();
           
           // Length units
           units.insert("meter".to_string(), Unit::meter());
           units.insert("m".to_string(), Unit::meter());
           units.insert("foot".to_string(), Unit::foot());
           // ... many more
           
           UnitDatabase { units }
       }
   }
   ```

3. **Update Parser Integration:**
   Modify your parser to look up units in the database instead of hardcoding them.

4. **(Advanced) Parse GNU units.dat:**
   ```rust
   // This is complex - consider it a stretch goal
   fn parse_units_dat(file_path: &str) -> Result<HashMap<String, Unit>, ParseError> {
       // Parse the GNU units format
       // This requires understanding their specific syntax
   }
   ```

#### **Validation Checklist:**
- [ ] Database contains 50+ units with proper conversions
- [ ] Parser uses database for unit lookup
- [ ] Supports unit aliases (m, meter, metres, etc.)
- [ ] Memory usage is reasonable
- [ ] Lookup performance is acceptable

#### **Rust Concepts You'll Learn:**
- **Static Initialization:** `lazy_static` and `once_cell` patterns
- **File I/O:** Reading and parsing external files
- **HashMap Performance:** Understanding lookup costs
- **Memory Management:** When to clone vs. reference data

---

### **Phase 5: Polish and Advanced Features**

**Goal:** Transform your functional tool into a polished application with great user experience.

#### **Tasks:**

1. **Interactive Mode (REPL):**
   ```rust
   use rustyline::Editor;

   fn interactive_mode() -> Result<(), Box<dyn Error>> {
       let mut rl = Editor::<()>::new()?;
       
       loop {
           let readline = rl.readline("runits> ");
           match readline {
               Ok(line) => {
                   // Parse and execute the command
               },
               Err(_) => break,
           }
       }
       Ok(())
   }
   ```

2. **Enhanced Error Messages:**
   ```rust
   // Instead of "Unknown unit: xyz"
   // Provide: "Unknown unit 'xyz'. Did you mean: meter, metre, m?"
   
   fn suggest_units(input: &str, database: &UnitDatabase) -> Vec<String> {
       // Implement fuzzy string matching
   }
   ```

3. **Additional Features:**
   - Unit prefixes (k, M, G, m, µ, etc.)
   - Currency conversion (requires API calls)
   - Temperature conversions (non-linear)
   - Better number formatting

#### **Code Quality Improvements:**
- Add comprehensive documentation with `///`
- Create a proper library API (`lib.rs`)
- Add benchmarks for performance testing
- Set up continuous integration

#### **Validation Checklist:**
- [ ] Interactive mode works smoothly
- [ ] Error messages are helpful and user-friendly
- [ ] Documentation is complete and helpful
- [ ] Performance is acceptable for large unit databases
- [ ] Code is well-organized and maintainable

#### **Rust Concepts You'll Learn:**
- **Closures:** Used extensively with interactive libraries
- **Traits:** Define common behavior across types
- **Documentation:** Using rustdoc effectively
- **Performance Profiling:** `cargo bench` and profiling tools
- **Advanced Error Handling:** Error chains and context

---

## **Testing Strategy Throughout**

### **Unit Tests** (Each Phase)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test individual functions and methods
    #[test] 
    fn test_unit_creation() { /* ... */ }
    
    #[test]
    fn test_conversion_logic() { /* ... */ }
}
```

### **Integration Tests** (Phase 2+)
```rust
// tests/integration_tests.rs
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn test_cli_basic_usage() {
    let mut cmd = Command::cargo_bin("runits").unwrap();
    cmd.arg("10").arg("meter").arg("foot");
    cmd.assert().success();
}
```

### **Property-Based Testing** (Advanced)
```rust
use quickcheck_macros::quickcheck;

#[quickcheck]
fn test_conversion_round_trip(value: f64) -> bool {
    let meter = Quantity::new(value, Unit::meter());
    let foot = meter.convert_to(&Unit::foot()).unwrap();
    let back_to_meter = foot.convert_to(&Unit::meter()).unwrap();
    
    (meter.value - back_to_meter.value).abs() < 1e-10
}
```

---

## **Phase Completion Checklist Template**

For each phase, ensure:
- [ ] Core functionality works as specified
- [ ] All tests pass (`cargo test`)
- [ ] Code compiles without warnings (`cargo check`)
- [ ] Clippy is happy (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] Git commits are clean and descriptive

---

## **Debugging Tips**

- **Use `dbg!()` liberally:** It's better than println! for debugging
- **Simplify when stuck:** Break complex problems into smaller pieces  
- **Read compiler errors carefully:** Rust's error messages are usually helpful
- **Don't fight the borrow checker:** If it's hard, there's probably a simpler way
- **Ask for help:** Rust community is very welcoming to beginners

This plan should now be much more approachable for Rust beginners while still covering all the important concepts. The key is to build confidence through small wins before tackling the more challenging aspects!