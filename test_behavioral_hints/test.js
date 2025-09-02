function riskyFunction(userInput) {
    // Unhandled promise
    fetchData()
        .then(data => processData(data));
    // Missing .catch()
    
    // Another unhandled promise
    asyncOperation()
        .then(result => {
            console.log(result);
        });
    
    // Promise with catch (handled)
    anotherOperation()
        .then(data => data)
        .catch(err => console.error(err));
    
    // Console.error usage
    if (userInput.length > 100) {
        console.error("Input too long");
    }
    
    // Throw statement
    if (!userInput) {
        throw new Error("Invalid input");
    }
    
    // TODO: Remove eval usage
    
    // Dangerous eval
    eval(userInput);
    
    // New Function (also dangerous)
    const fn = new Function('x', userInput);
    
    // Process exit
    if (userInput === 'exit') {
        process.exit(1);
    }
    
    // FIXME: Security vulnerability
}

// Dummy functions
async function fetchData() { return {}; }
async function processData(data) { return data; }
async function asyncOperation() { return true; }
async function anotherOperation() { return {}; }