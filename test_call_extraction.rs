// Test file to understand call graph extraction behavior

fn outer_function() {
    println!("In outer");
    inner_function();
    another_function();
}

fn inner_function() {
    println!("In inner");
    deeply_nested();
}

fn another_function() {
    println!("In another");
    helper();
}

fn deeply_nested() {
    println!("Deep");
    helper();
}

fn helper() {
    println!("Helper");
}

struct MyStruct {
    value: i32,
}

impl MyStruct {
    fn new() -> Self {
        Self { value: 0 }
    }
    
    fn method_one(&self) {
        self.method_two();
        helper();
    }
    
    fn method_two(&self) {
        println!("Method two");
    }
}

fn main() {
    outer_function();
    
    let s = MyStruct::new();
    s.method_one();
}