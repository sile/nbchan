nbchan
======

Informal Benchmarks
--------------------

The version of `nbchan` is `v0.1.0`:

```sh
$ cat /etc/lsb-release | tail -1
DISTRIB_DESCRIPTION="Ubuntu 17.04"

$ cat /proc/cpuinfo | grep 'model name'
model name      : Intel(R) Core(TM) i7-6600U CPU @ 2.60GHz
model name      : Intel(R) Core(TM) i7-6600U CPU @ 2.60GHz
model name      : Intel(R) Core(TM) i7-6600U CPU @ 2.60GHz
model name      : Intel(R) Core(TM) i7-6600U CPU @ 2.60GHz

$ rustup run nightly rustc -V
rustc 1.18.0-nightly (036983201 2017-04-26)

$ rustup run nightly cargo bench
running 8 tests
test create_nbchan_oneshot                ... bench:          23 ns/iter (+/- 1)
test create_std_mpsc                      ... bench:          64 ns/iter (+/- 6)
test failure_send_nbchan_oneshot          ... bench:          40 ns/iter (+/- 1)
test failure_send_std_mpsc                ... bench:          85 ns/iter (+/- 6)
test multithread_send_recv_nbchan_oneshot ... bench:          71 ns/iter (+/- 19)
test multithread_send_recv_std_mpsc       ... bench:         108 ns/iter (+/- 53)
test send_recv_nbchan_oneshot             ... bench:          35 ns/iter (+/- 4)
test send_recv_std_mpsc                   ... bench:          82 ns/iter (+/- 3)

test result: ok. 0 passed; 0 failed; 0 ignored; 8 measured
```

```sh
$ cargo run --example channel_size
nbchan::oneshot::Sender<()>:   8 bytes
nbchan::oneshot::Receiver<()>: 8 bytes
std::sync::mpsc::Sender<()>:   16 bytes
std::sync::mpsc::Receiver<()>: 16 bytes
```
