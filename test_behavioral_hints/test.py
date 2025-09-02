import sys
import os
import threading

def risky_function(user_input):
    """Function with various behavioral hints"""
    
    # Bare except clause
    try:
        result = some_operation()
    except:
        pass
    
    # Another bare except
    try:
        another_operation()
    except Exception:
        pass
    
    # Dangerous eval
    try:
        eval(user_input)
    except ValueError:
        # TODO: Remove eval usage
        print("Error in eval")
    
    # System exit
    if len(user_input) > 1000:
        sys.exit(1)
    
    # FIXME: This is not secure
    exec(user_input)

def concurrent_function():
    """Function with threading"""
    lock = threading.Lock()
    
    with lock:
        # Critical section
        print("In critical section")
    
    # Using RLock
    rlock = threading.RLock()
    with rlock:
        print("In reentrant lock")