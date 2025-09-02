package main

import (
    "os"
    "sync"
    "unsafe"
)

func riskyFunction(data string) {
    // Ignored error
    result, _ := ReadFile("config.json")
    
    // Another ignored error with explicit underscore
    _ = ProcessData(result)
    
    // Ignored error in assignment
    file, _ := os.Open("test.txt")
    defer file.Close()
    
    // Panic call
    if len(data) > 1000 {
        panic("data too large")
    }
    
    // TODO: Handle errors properly
    
    // OS exit
    if data == "exit" {
        os.Exit(1)
    }
    
    // Unsafe usage
    ptr := unsafe.Pointer(&data)
    _ = ptr
}

func concurrentFunction() {
    // Mutex usage
    var mutex sync.Mutex
    mutex.Lock()
    defer mutex.Unlock()
    
    // RWMutex usage
    var rwMutex sync.RWMutex
    rwMutex.RLock()
    defer rwMutex.RUnlock()
    
    // FIXME: Potential deadlock
}

// Dummy functions for compilation
func ReadFile(path string) ([]byte, error) {
    return nil, nil
}

func ProcessData(data []byte) error {
    return nil
}