use alloc::sync::Arc;
use core::ops::Deref;
use std::sync::RwLock;

use arcu::{epoch_counters::EpochCounter, Rcu};

extern crate alloc;

#[cfg(all(feature = "global_counters", feature = "thread_local_counter"))]
struct Loud<T: core::fmt::Debug>(T);

#[cfg(all(feature = "global_counters", feature = "thread_local_counter"))]
impl<T: core::fmt::Debug> Drop for Loud<T> {
    fn drop(&mut self) {
        println!("Dropping: {:?}", self.0);
    }
}

#[cfg(all(feature = "global_counters", feature = "thread_local_counter"))]
#[test]
fn std_replace() {
    use arcu::epoch_counters::GlobalEpochCounterPool;

    let rcu = arcu::atomic::Arcu::new(Loud(11), GlobalEpochCounterPool);
    assert_eq!(rcu.read().0, 11);
    rcu.replace(Loud(55));
    assert_eq!(rcu.read().0, 55);
}

#[cfg(all(feature = "global_counters", feature = "thread_local_counter"))]
#[test]
fn std_update() {
    use arcu::epoch_counters::GlobalEpochCounterPool;

    let rcu = arcu::atomic::Arcu::new(Loud((0, 0)), GlobalEpochCounterPool);
    let rcu_ref = &rcu;
    assert_eq!(rcu.read().0, (0, 0));

    std::thread::scope(|scope| {
        for idx in 0..100 {
            scope
                .spawn(move || rcu_ref.try_update(|old| Some(Arc::new(Loud((idx, old.0 .1 + 1))))));
        }
    });

    assert_eq!(rcu.read().0 .1, 100);
}

#[test]
fn raw_replace_atomic() {
    raw_replace::<arcu::atomic::Arcu<_, _>>()
}

#[test]
fn raw_replace_rwlock() {
    raw_replace::<arcu::rwlock::Arcu<_, _>>()
}

fn raw_replace<Arcu: Rcu<Item = i32, Pool = [Arc<EpochCounter>; 100]> + Send + Sync>() {
    let epoch_counters: [_; 100] = std::array::from_fn(|_| Arc::new(EpochCounter::new()));

    let rcu = Arcu::new(201, epoch_counters.clone());

    let val = unsafe { rcu.raw_read(&epoch_counters[0]) };
    assert_eq!(val.deref(), &201);

    let epoch_counters: &_ = &epoch_counters;

    std::thread::scope(|scope| {
        for idx in 0..100 {
            let new = Arc::new(idx);
            scope.spawn(|| {
                let to_drop = rcu.replace(new);
                println!("Dropping: {to_drop}");
            });
        }
    });

    let val = unsafe { rcu.raw_read(&epoch_counters[0]) };
    assert!((0..100).contains(val.deref()));
}

#[test]
fn raw_update1_atomic() {
    raw_update1::<arcu::atomic::Arcu<_, _>>()
}

#[test]
fn raw_update1_rwlock() {
    raw_update1::<arcu::rwlock::Arcu<_, _>>()
}

fn raw_update1<Arcu: Rcu<Item = RwLock<usize>, Pool = [Arc<EpochCounter>; 100]> + Send + Sync>() {
    let epoch_counters: [_; 100] = std::array::from_fn(|_| Arc::new(EpochCounter::new()));
    let mut idx = 0;
    let epoch_counters_plus: [_; 100] = epoch_counters.clone().map(|counter| {
        (
            counter,
            Arc::new(RwLock::new({
                let old = idx;
                idx += 1;
                old
            })),
        )
    });

    let rcu = Arcu::new(RwLock::new(0), epoch_counters.clone());

    let epoch_counters_ref: &_ = &epoch_counters_plus;

    std::thread::scope(|scope| {
        for (epoch_counter, arc) in epoch_counters_ref {
            scope.spawn(|| {
                let to_drop = unsafe {
                    rcu.raw_try_update(
                        |old| {
                            let old = *old.read().unwrap();
                            println!("Old: {old}");
                            *arc.write().unwrap() = old + 1;
                            Some(arc.clone())
                        },
                        epoch_counter.deref(),
                    )
                };
                if let Some(to_drop) = to_drop {
                    let to_drop = *to_drop.read().unwrap();
                    println!("Dropping: {to_drop}");
                }
            });
        }
    });

    let final_val = unsafe { rcu.raw_read(&epoch_counters_ref[0].0) };

    assert_eq!(final_val.read().unwrap().deref(), &epoch_counters_ref.len());

    drop(epoch_counters);
}

#[test]
fn raw_update2_atomic() {
    raw_update2::<arcu::atomic::Arcu<_, _>>()
}

#[test]
fn raw_update2_rwlock() {
    raw_update2::<arcu::rwlock::Arcu<_, _>>()
}

fn raw_update2<Arcu: Rcu<Item = usize, Pool = [Arc<EpochCounter>; 100]> + Send + Sync>() {
    let epoch_counters: [_; 100] = std::array::from_fn(|_idx| Arc::new(EpochCounter::new()));
    let rcu = Arcu::new(Arc::new(0), epoch_counters.clone());

    let epoch_counters_ref: &_ = &epoch_counters;

    std::thread::scope(|scope| {
        for epoch_counter in epoch_counters_ref {
            scope.spawn(|| {
                let to_drop = unsafe {
                    rcu.raw_try_update(
                        |old: &usize| {
                            println!("Old: {old}");
                            Some(Arc::new(old + 1))
                        },
                        epoch_counter.deref(),
                    )
                };
                if let Some(to_drop) = to_drop {
                    println!("Dropping: {to_drop}");
                }
            });
        }
    });

    let final_val = unsafe { rcu.raw_read(&epoch_counters_ref[0]) };

    assert_eq!(final_val.deref(), &epoch_counters_ref.len());

    drop(epoch_counters);
}
