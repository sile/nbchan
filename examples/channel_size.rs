extern crate nbchan;

use std::mem;

fn main() {
    let (tx, rx) = nbchan::oneshot::channel::<()>();
    println!("nbchan::oneshot::Sender<()>:   {} bytes",
             mem::size_of_val(&tx));
    println!("nbchan::oneshot::Receiver<()>: {} bytes",
             mem::size_of_val(&rx));

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    println!("std::sync::mpsc::Sender<()>:   {} bytes",
             mem::size_of_val(&tx));
    println!("std::sync::mpsc::Receiver<()>: {} bytes",
             mem::size_of_val(&rx));
}
