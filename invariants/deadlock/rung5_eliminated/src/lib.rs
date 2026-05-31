//! Rung 5: eliminated. The hazard cannot form because there is no second lock.
//!
//! Rungs 1 to 4 leave two locks in the program and argue about how the order
//! between them is enforced. Rung 5 deletes the question. The three counters are
//! owned by a single thread, and every other thread interacts only by sending it
//! a message. One place touches the data, so there is no acquisition order to get
//! wrong, and no Mutex in this crate at all.
//!
//! This is the elimination control: not a guard on the hazard, the removal of
//! it. The cost moves. No lock order bug is possible, but you pay in a
//! restructured design (an owner thread, a command protocol, channel latency on
//! the hot path) and give up shared memory ergonomics.

use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};

// BOILERPLATE-START rung5_actor
// Messages are the only way to reach the state. No lock is exposed.
enum Command {
    Tick(Sender<u64>),
    Shutdown,
}

pub struct Bank {
    tx: Sender<Command>,
    worker: Option<JoinHandle<()>>,
}

impl Bank {
    pub fn new() -> Self {
        let (tx, rx) = channel::<Command>();
        let worker = thread::spawn(move || {
            // the one owner of the state, exclusive by construction not by lock
            let (mut accounts_table, mut account, mut audit_log) = (0u64, 0u64, 0u64);
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Command::Tick(reply) => {
                        accounts_table += 1;
                        account += 1;
                        audit_log += 1;
                        let _ = reply.send(accounts_table + account + audit_log);
                    }
                    Command::Shutdown => break,
                }
            }
        });
        Bank { tx, worker: Some(worker) }
    }

    // Crosses a thread boundary (a channel round trip), a different cost class
    // from rungs 1 to 4, which is why the runtime column lists rung 5 as n/a.
    pub fn hot_path(&self) -> u64 {
        let (reply_tx, reply_rx) = channel();
        self.tx.send(Command::Tick(reply_tx)).expect("owner thread alive");
        reply_rx.recv().expect("owner replies")
    }
}
// BOILERPLATE-END rung5_actor

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Bank {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // No second lock to acquire, so the rungs 1 to 4 hazard cannot form. Hammer
    // the single owner from many threads in arbitrary timing; every call
    // completes and the final state is exactly the number of ticks. Nothing here
    // can hang on lock order.
    #[test]
    fn hazard_cannot_form_single_owner() {
        let bank = Arc::new(Bank::new());
        let threads = 8;
        let per_thread = 1000;
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let bank = bank.clone();
                thread::spawn(move || {
                    for _ in 0..per_thread {
                        let _ = bank.hot_path();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().expect("worker never deadlocks");
        }
        let total_ticks = (threads * per_thread) as u64;
        assert_eq!(bank.hot_path(), 3 * (total_ticks + 1));
    }
}
