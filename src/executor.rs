use core::{
    future::Future,
    pin::Pin,
    ptr::null,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use alloc::{boxed::Box, collections::LinkedList, sync::Arc, vec::Vec};

use crate::{
    sync::Mutex,
    thread::{self, JoinHandle},
};

pub struct Executor {
    workers: Pin<Arc<Mutex<Vec<Worker>>>>,
    in_queue: Pin<Arc<Mutex<Option<LinkedList<Task>>>>>,
}

struct Worker {
    handle: JoinHandle<()>,
    queue: Pin<Arc<Mutex<Option<LinkedList<Task>>>>>,
}

struct Task {
    fut: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
}

impl Executor {
    pub fn block_on<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + Sync + 'static,
        F::Output: Send + Sync,
    {
        // The Mutex is `Pin` in it's `Arc`
        let result: Pin<Arc<Mutex<Option<F::Output>>>> = unsafe { Arc::pin(Mutex::new(None)) };

        {
            let result = result.clone();

            self.spawn(async move {
                let res = fut.await;

                *result.lock() = Some(res);
            });
        }

        // TODO: can this wake up spuriously?
        result.wait();

        result
            .lock()
            .consume()
            .expect("task result futex was unlocked, but no value was returned")
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + Sync + 'static,
    {
        self.in_queue
            .lock()
            .as_mut()
            .unwrap()
            .push_back(Task { fut: Box::pin(fut) });
    }
}

const WORKER_STACK_SIZE: usize = 1024 * 1024;

fn worker(
    id: usize,
    my_queue: Pin<Arc<Mutex<Option<LinkedList<Task>>>>>,
    siblings: Pin<Arc<Mutex<Vec<Worker>>>>,
) {
    #[allow(clippy::while_let_loop)]
    'work: loop {
        let work = if let Some(my_queue) = my_queue.lock().as_mut() {
            my_queue.pop_front()
        } else {
            break;
        };

        let work = if let Some(work) = work {
            work
        } else {
            let n = siblings.lock().len();

            'siblings: for i in 0..n {
                if i == id {
                    continue 'siblings;
                }

                let sibling_queue = siblings.lock()[i].queue.clone();

                let mut queue_guard = sibling_queue.lock();

                let queue = if let Some(queue) = queue_guard.as_mut() {
                    queue
                } else {
                    continue;
                };
                let n = queue.len();
                if n > 1 {
                    let mut new_work = queue.split_off(n / 2);

                    drop(queue_guard);

                    if let Some(my_queue) = my_queue.lock().as_mut() {
                        my_queue.append(&mut new_work)
                    } else {
                        break 'work;
                    };
                    continue 'work;
                }
            }

            // No work. Wait for new work to be added

            let in_queue = if let Some(worker) = siblings.lock().get(0) {
                worker.queue.clone()
            } else {
                break 'work;
            };

            in_queue.wait();

            continue;
        };

        trace!("worker got work");

        unsafe {
            let mut work = work;

            unsafe fn wake(_ptr: *const ()) {
                todo!("RawWakerVTable: wake");
            }
            unsafe fn wake_by_ref(_ptr: *const ()) {
                todo!("RawWakerVTable: wake_by_ref");
            }
            unsafe fn drop(_ptr: *const ()) {
                // todo!("RawWakerVTable: drop");
            }
            unsafe fn clone(_ptr: *const ()) -> RawWaker {
                todo!("RawWakerVTable: clone");
            }

            static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

            let raw_waker = RawWaker::new(null(), &VTABLE);

            let waker = Waker::from_raw(raw_waker);

            let mut context = Context::from_waker(&waker);

            trace!("polling");

            let res = Future::poll(work.fut.as_mut(), &mut context);

            match res {
                core::task::Poll::Ready(_) => {
                    trace!("finished an async task");
                    continue;
                }
                core::task::Poll::Pending => {
                    if let Some(my_queue) = my_queue.lock().as_mut() {
                        my_queue.push_back(work);
                    } else {
                        break 'work;
                    }

                    todo!()
                }
            }
        }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        let n = self.workers.lock().len();

        let mut workers = Vec::with_capacity(n);
        for _ in 0..n {
            let worker = self.workers.lock().pop().unwrap();

            let q = worker.queue.lock().take();

            if let Some(q) = q {
                if !q.is_empty() {
                    trace!("dropping {} tasks", q.len());
                }
            }

            workers.push(worker);
        }

        for worker in workers {
            worker.handle.join().expect("Failed to join worker thread");
        }
    }
}

pub fn init(n_threads: usize) -> Executor {
    // Safety: the Arc `Pin`s the Mutex
    let workers = unsafe { Arc::pin(Mutex::new(Vec::with_capacity(n_threads))) };

    for i in 0..n_threads {
        // Safety: the Arc `Pin`s the Mutex
        let queue = unsafe { Arc::pin(Mutex::new(Some(LinkedList::new()))) };

        let handle = {
            let queue = queue.clone();
            let workers = workers.clone();

            unsafe {
                thread::spawn(move || worker(i, queue, workers), WORKER_STACK_SIZE)
                    .expect("Failed to spawn worker thread")
            }
        };

        workers.lock().push(Worker { handle, queue });
    }

    let in_queue = workers.lock()[0].queue.clone();

    Executor { workers, in_queue }
}
