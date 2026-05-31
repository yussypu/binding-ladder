//! Rung 5 — eliminated. The hazard cannot form because there is no second lock.
//!
//! Rungs 1–4 all leave two locks in the program and argue about how the order
//! between them is enforced (a comment, a detector, a type). Rung 5 deletes the
//! question: the three counters are owned by a *single* thread, and every other
//! thread interacts only by sending it a message. There is exactly one place
//! that touches the data, so there is no acquisition *order* to get wrong — and
//! in fact no `Mutex` in this crate at all (assert it: `grep -c Mutex src` == 0).
//!
//! This is the NIOSH "elimination" control: not a guard on the hazard, the
//! removal of the hazard. The cost moves: no lock-order bug is possible, but you
//! pay in a restructured design (an owner thread, a command protocol, channel
//! latency on the hot path) and you give up shared-memory ergonomics.

use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};

// BOILERPLATE-START rung5_actor (caller-authored: the restructure — owner thread + protocol)
/// Messages are the only way to reach the state. No lock is exposed.
enum Command {
    /// Bump all three counters; reply with their sum.
    Tick(Sender<u64>),
    Shutdown,
}

/// A handle to the single-owner bank. Cloneable senders could fan in from many
/// threads; none of them can hold a lock, because there is none to hold.
pub struct Bank {
    tx: Sender<Command>,
    worker: Option<JoinHandle<()>>,
}

impl Bank {
    pub fn new() -> Self {
        let (tx, rx) = channel::<Command>();
        let worker = thread::spawn(move || {
            // The ONE owner of the state. Exclusive by construction, not by lock.
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

    /// API-comparable hot path: ask the owner to do the work and report the sum.
    /// Crosses a thread boundary (channel round-trip) — a different cost *class*
    /// than rungs 1–4, which is why the runtime column lists rung 5 as n/a.
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

    /// THE POINT OF RUNG 5: there is no second lock to acquire, so the deadlock
    /// hazard from rungs 1–4 cannot form. We hammer the single owner from many
    /// threads in arbitrary timing; every call completes (no thread can be stuck
    /// waiting on a lock another holds, because there are no locks), and the
    /// final state is exactly the number of ticks. No watchdog needed — nothing
    /// here can hang on lock order.
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
                        // returns promptly; cannot deadlock — no lock to misorder
                        let _ = bank.hot_path();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().expect("worker never deadlocks");
        }
        // After N ticks each counter == N, so the sum the owner reports == 3*N.
        let total_ticks = (threads * per_thread) as u64;
        assert_eq!(bank.hot_path(), 3 * (total_ticks + 1));
    }
}
