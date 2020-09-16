#[macro_use]
extern crate jez;

use std::fs;
use std::io;
use std::io::Read;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

use docopt::Docopt;
use serde::Deserialize;

use jez::{simulate, Backend, Command, Error, Machine, Program, Sink, Status};

const USAGE: &'static str = "
Jez.

Usage:
  jez [options] info
  jez [options] [<file>]
  jez (-h | --help)
  jez --version

Options:
  -h, --help            Show this screen.
  --version             Show version.
  --verbose             Print more output.
  --watch               Reload input file on changes.
  --simulate            Run as a non-realtime simulation.
  --time=MS             Length of time (in milliseconds) to run for.
  --sink=NAME           Specify the output sink(s).
  --udp-host=ADDRESS    UDP host address [default: 127.0.0.1:34254].
  --udp-client=ADDRESS  UDP client address [default: 127.0.0.1:3000].
  --midi-out=DEVICE     Midi output device id.
  --ws-host=ADDRESS     Websocket host address [default: 127.0.0.1:2794].

Sinks:
  console
  portmidi
  udp
  websocket
  null
  renoise
";

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct Args {
    flag_sink: String,
    flag_time: String,
    flag_simulate: bool,
    flag_watch: bool,
    flag_verbose: bool,
    flag_version: bool,
    flag_udp_host: String,
    flag_udp_client: String,
    flag_midi_out: Option<usize>,
    flag_ws_host: String,
    arg_file: String,
    cmd_info: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TaskStatus {
    Continue,
    Completed,
}

type Task = Box<dyn FnMut() -> Result<TaskStatus, Error> + Send>;

fn watcher_task(
    filepath: String,
    program: Program,
    channel: Sender<Command>,
) -> Result<Task, Error> {
    let meta_data = fs::metadata(filepath.clone())?;
    let mod_time = meta_data.modified()?;

    Ok(Box::new(move || {
        let new_meta_data = fs::metadata(filepath.clone())?;
        let new_mod_time = new_meta_data.modified()?;

        if new_mod_time != mod_time {
            let mut txt = String::new();
            let mut fp = fs::File::open(filepath.clone())?;
            fp.read_to_string(&mut txt)?;

            if program != Program::new(txt.as_str())? {
                channel.send(Command::Reload).unwrap();
                return Ok(TaskStatus::Completed);
            }
        }

        Ok(TaskStatus::Continue)
    }))
}

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

fn make_sink(names: &str, args: &Args) -> Result<Sink, Error> {
    let mut requests = vec![];
    for name in names.split(',') {
        requests.push(match name {
            "null" | "" => Backend::Null,
            "console" => Backend::Console,
            "udp" => Backend::Udp(&args.flag_udp_host, &args.flag_udp_client),
            "renoise" => Backend::Renoise(&args.flag_udp_host, &args.flag_udp_client),
            "portmidi" => Backend::PortMidi(args.flag_midi_out),
            "websocket" => Backend::WebSocket(&args.flag_ws_host),
            _ => return Err(error!(UnknownBackend, name)),
        });
    }
    Sink::new(&requests)
}

fn read_program(file_path: &str) -> Result<String, Error> {
    let mut txt = String::new();
    if file_path.is_empty() {
        io::stdin().read_to_string(&mut txt)?;
    } else {
        let mut fp = fs::File::open(file_path)?;
        fp.read_to_string(&mut txt)?;
    }
    Ok(txt)
}

fn run_app(args: &Args) -> Result<(), Error> {
    if args.flag_simulate {
        let txt = read_program(&args.arg_file)?;
        let dur = if args.flag_time.is_empty() {
            60000.0
        } else {
            match args.flag_time.parse::<f64>() {
                Ok(time) => time,
                Err(_) => return Err(error!(InvalidArgs, "Invalid time")),
            }
        };
        let data = simulate(dur, 0.5, &txt)?;
        println!("{}", data);
        return Ok(());
    }

    let mut sink = make_sink(&args.flag_sink, &args)?;

    if args.cmd_info {
        println!("Sink: {}", sink.name());
        let devices = sink.devices();
        for dev in &devices {
            println!("{}", dev);
        }
        return Ok(());
    }

    let (sink_send, sink_recv) = channel();
    sink.run_forever(sink_recv);

    loop {
        let txt = read_program(&args.arg_file)?;
        let program = Program::new(txt.as_str())?;

        let (host_to_mach_send, host_to_mach_recv) = channel();

        let mut tasks: Vec<Task> = vec![];
        if args.flag_watch && !args.arg_file.is_empty() {
            let task = watcher_task(
                args.arg_file.clone(),
                program.clone(),
                host_to_mach_send.clone(),
            )?;
            tasks.push(task);
        }

        let mach_to_sink_send = sink_send.clone();
        let mut machine = Machine::new(
            &program,
            Box::new(move || host_to_mach_recv.try_recv().ok()),
            Box::new(move |cmd| mach_to_sink_send.send(cmd).unwrap_or(())),
        )?;

        if !args.flag_time.is_empty() {
            match args.flag_time.parse::<f64>() {
                Ok(time) => machine.schedule(time, Command::Stop),
                Err(_) => return Err(error!(InvalidArgs, "Invalid time")),
            }
        }

        if !tasks.is_empty() {
            thread::spawn(move || run_until_first(tasks));
        }

        match machine.run_forever()? {
            Status::Stop => return Ok(()),
            Status::Reload | Status::Continue => (),
        };

        if args.flag_verbose {
            println!("Reloading {}", args.arg_file);
        }
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("v{}", VERSION);
        return;
    }

    let code = match run_app(&args) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    };

    std::process::exit(code);
}
