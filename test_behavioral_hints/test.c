#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

void risky_function(char* input) {
    // Unchecked memory allocation
    char* buffer = malloc(100);
    
    // Unsafe string copy
    strcpy(buffer, input);
    
    // TODO: Fix memory leak - no free() call
    
    // Assertion
    assert(buffer != NULL);
    
    // Abrupt exit
    if (strlen(input) > 100) {
        abort();
    }
}

void thread_function() {
    pthread_mutex_t mutex;
    pthread_mutex_init(&mutex, NULL);
    
    // FIXME: Should handle errors
    pthread_mutex_lock(&mutex);
    
    // Critical section
    printf("In critical section\n");
    
    pthread_mutex_unlock(&mutex);
    pthread_mutex_destroy(&mutex);
}