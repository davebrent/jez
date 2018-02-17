extern crate docopt;
extern crate jez;
#[macro_use]
extern crate serde_derive;

use std::convert::From;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

use docopt::Docopt;

use jez::{make_program, make_sink, millis_to_dur, Command, Control, Instr, JezErr, Machine,
          RuntimeErr, SinkArgs};

const USAGE: &str = "
Jez.

Usage:
  jez [options] list
  jez [options] [<file>]
  jez (-h | --help)
  jez --version

Options:
  -h, --help            Show this screen.
  --version             Show version.
  --verbose             Print more output.
  --watch               Reload input file on changes.
  --time=MS             Length of time (in milliseconds) to run for.
  --sink=NAME           Specify the output sink(s) [default: console].
  --osc-host=ADDRESS    OSC host address [default: 127.0.0.1:34254].
  --osc-client=ADDRESS  OSC client address [default: 127.0.0.1:3000].
  --midi-out=DEVICE     Midi output device id.
  --ws-host=ADDRESS     Websocket host address [default: 127.0.0.1:2794].

Sinks:
  console
  portmidi
  osc
  websocket
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_sink: String,
    flag_time: String,
    flag_watch: bool,
    flag_verbose: bool,
    flag_version: bool,
    flag_osc_host: String,
    flag_osc_client: String,
    flag_midi_out: Option<usize>,
    flag_ws_host: String,
    arg_file: String,
    cmd_list: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TaskStatus {
    Continue,
    Completed,
}

type Task = Box<FnMut() -> Result<TaskStatus, JezErr> + Send>;

/// Send a `stop` command after a specified period of time
fn timer_task(millis: f64, channel: Sender<Command>) -> Task {
    let start = Instant::now();
    let end = millis_to_dur(millis);

    Box::new(move || {
        if start.elapsed() >= end {
            channel.send(Command::Stop).unwrap();
            Ok(TaskStatus::Completed)
        } else {
            Ok(TaskStatus::Continue)
        }
    })
}

/// Send a `reload` command when a program file changes
fn watcher_task(
    filepath: String,
    instrs: Vec<Instr>,
    channel: Sender<Command>,
) -> Result<Task, JezErr> {
    let meta_data = try!(fs::metadata(filepath.clone()));
    let mod_time = try!(meta_data.modified());

    Ok(Box::new(move || {
        let new_meta_data = try!(fs::metadata(filepath.clone()));
        let new_mod_time = try!(new_meta_data.modified());

        if new_mod_time != mod_time {
            let mut txt = String::new();
            let mut fp = try!(fs::File::open(filepath.clone()));
            try!(fp.read_to_string(&mut txt));

            if instrs != try!(make_program(txt.as_str())) {
                channel.send(Command::Reload).unwrap();
                return Ok(TaskStatus::Completed);
            }
        }

        Ok(TaskStatus::Continue)
    }))
}

/// Run all tasks until one is completed
fn run_until_first(tasks: Vec<Task>) {
    let mut tasks = tasks;
    let res = Duration::new(0, 1_000_000); // 1ms

    'outer: loop {
        for task in &mut tasks {
            let status = match task() {
                Ok(status) => status,
                Err(_) => break 'outer,
            };
            match status {
                TaskStatus::Continue => (),
                TaskStatus::Completed => break 'outer,
            };
        }
        thread::sleep(res);
    }
}

fn run_app(args: &Args) -> Result<(), JezErr> {
    let sink_args = SinkArgs::new(
        &args.flag_osc_host,
        &args.flag_osc_client,
        &args.flag_ws_host,
        args.flag_midi_out,
    );

    let mut sink = try!(make_sink(&args.flag_sink, &sink_args));

    if args.cmd_list {
        let devices = sink.devices();
        for dev in &devices {
            println!("{}", dev);
        }
        return Ok(());
    }

    let (sink_send, sink_recv) = channel();
    sink.run_forever(sink_recv);

    loop {
        let mut txt = String::new();
        if args.arg_file.is_empty() {
            try!(io::stdin().read_to_string(&mut txt));
        } else {
            let mut fp = try!(fs::File::open(args.arg_file.clone()));
            try!(fp.read_to_string(&mut txt));
        }

        let instrs = try!(make_program(txt.as_str()));

        let (host_send, host_recv) = channel();
        let mut tasks: Vec<Task> = vec![];

        if args.flag_watch && !args.arg_file.is_empty() {
            let task = try!(watcher_task(
                args.arg_file.clone(),
                instrs.clone(),
                host_send.clone(),
            ));
            tasks.push(task);
        }

        if !args.flag_time.is_empty() {
            match args.flag_time.parse::<f64>() {
                Ok(time) => {
                    let task = timer_task(time, host_send.clone());
                    tasks.push(task);
                }
                Err(_) => {
                    let msg = String::from("Invalid time argument");
                    let err = RuntimeErr::InvalidArgs(Some(msg));
                    return Err(From::from(err));
                }
            }
        }

        if !tasks.is_empty() {
            thread::spawn(move || run_until_first(tasks));
        }

        let mut machine = Machine::new(sink_send.clone(), host_send.clone(), host_recv, &instrs);

        match try!(machine.exec_realtime()) {
            Control::Stop => return Ok(()),
            _ => {
                if args.flag_verbose {
                    println!("Reloading {}", args.arg_file);
                }
            }
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("v0.6.0");
        return;
    }

    let code = match run_app(&args) {
        Ok(_) => 0,
        Err(err) => {
            writeln!(io::stderr(), "Error: {}", err).unwrap();
            1
        }
    };

    std::process::exit(code);
}
