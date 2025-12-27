use midir::{Ignore, MidiInput, MidiOutput};
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    ControlChange { controller: u8, value: u8 },
    Connected,
    Disconnected,
}

pub enum MidiCommand {
    SetPadColor { note: u8, color: u8 },
    SetButtonColor { cc: u8, color: u8 },
    ClearAll,
}

pub fn start_midi_service(tx_to_app: Sender<MidiEvent>) -> Sender<MidiCommand> {
    let (tx_cmd, rx_cmd) = std::sync::mpsc::channel();

    thread::spawn(move || {
        let mut first_attempt = true;
        loop {
            // Send disconnected status
            if !first_attempt {
                let _ = tx_to_app.send(MidiEvent::Disconnected);
            }

            match run_midi_loop(&tx_to_app, &rx_cmd) {
                Ok(_) => {
                    println!("MIDI service stopped normally");
                    break;
                }
                Err(e) => {
                    if first_attempt {
                        println!("MIDI: Waiting for Launchpad... ({:?})", e);
                        first_attempt = false;
                    } else {
                        println!("MIDI: Launchpad disconnected. Retrying... ({:?})", e);
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    });

    tx_cmd
}

fn run_midi_loop(
    tx_event: &Sender<MidiEvent>,
    rx_cmd: &Receiver<MidiCommand>,
) -> Result<(), Box<dyn Error>> {
    // Try to get ports, with multiple attempts to work around macOS threading issues
    // CoreMIDI requires a RunLoop (usually main thread) to populate device properties.
    // We'll retry a few times with delays to give the system a chance.

    let (midi_in, midi_out, in_ports, out_ports) = {
        let mut last_attempt = (None, None, vec![], vec![]);

        for attempt in 0..3 {
            if attempt > 0 {
                thread::sleep(Duration::from_millis(500));
            }

            let mut midi_in = MidiInput::new("Lightspeed Input")?;
            midi_in.ignore(Ignore::None);
            let midi_out = MidiOutput::new("Lightspeed Output")?;

            let in_ports = midi_in.ports();
            let out_ports = midi_out.ports();

            // Check if any ports have valid names
            let has_valid_name = in_ports.iter().any(|p| midi_in.port_name(p).is_ok())
                || out_ports.iter().any(|p| midi_out.port_name(p).is_ok());

            if has_valid_name {
                last_attempt = (Some(midi_in), Some(midi_out), in_ports, out_ports);
                break;
            }

            last_attempt = (Some(midi_in), Some(midi_out), in_ports, out_ports);
        }

        let (Some(midi_in), Some(midi_out), in_ports, out_ports) = last_attempt else {
            return Err("Failed to initialize MIDI".into());
        };

        (midi_in, midi_out, in_ports, out_ports)
    };

    // List all ports for debugging
    println!("\nAvailable Input Ports:");
    for (i, p) in in_ports.iter().enumerate() {
        match midi_in.port_name(p) {
            Ok(name) => println!("{}: '{}' (len: {})", i, name, name.len()),
            Err(e) => println!("{}: ERROR - {:?}", i, e),
        }
    }
    println!("\nAvailable Output Ports:");
    for (i, p) in out_ports.iter().enumerate() {
        match midi_out.port_name(p) {
            Ok(name) => println!("{}: '{}' (len: {})", i, name, name.len()),
            Err(e) => println!("{}: ERROR - {:?}", i, e),
        }
    }

    // Smart Selection Logic - STRICT name matching required
    // CRITICAL: We MUST skip any port where we can't retrieve the name.
    // Attempting to connect to a port in "zombie" state (CannotRetrievePortName)
    // poisons the connection. We let the retry loop wait for the port to fully initialize.
    //
    // 1. Prefer "Launchpad" AND "MIDI"
    // 2. Prefer "Launchpad" AND NOT "DAW"
    // 3. Fallback to any "Launchpad"

    // Find Input - STRICT: only use ports with valid, readable names
    let lp_in = in_ports.iter().find(|p| {
        // Skip ports where we can't get the name (device still initializing)
        let Ok(name) = midi_in.port_name(p) else {
            println!("Skipping initializing input device (name unavailable)");
            return false;
        };
        name.contains("Launchpad") && (name.contains("MIDI") || name.contains("LPMiniMK3 MIDI"))
    }).or_else(|| {
        in_ports.iter().find(|p| {
            let Ok(name) = midi_in.port_name(p) else {
                return false;
            };
            name.contains("Launchpad") && !name.contains("DAW")
        })
    }).or_else(|| {
        in_ports.iter().find(|p| {
            let Ok(name) = midi_in.port_name(p) else {
                return false;
            };
            name.contains("Launchpad")
        })
    });

    // Find Output - STRICT: only use ports with valid, readable names
    let lp_out = out_ports.iter().find(|p| {
        // Skip ports where we can't get the name (device still initializing)
        let Ok(name) = midi_out.port_name(p) else {
            println!("Skipping initializing output device (name unavailable)");
            return false;
        };
        name.contains("Launchpad") && (name.contains("MIDI") || name.contains("LPMiniMK3 MIDI"))
    }).or_else(|| {
        out_ports.iter().find(|p| {
            let Ok(name) = midi_out.port_name(p) else {
                return false;
            };
            name.contains("Launchpad") && !name.contains("DAW")
        })
    }).or_else(|| {
        out_ports.iter().find(|p| {
            let Ok(name) = midi_out.port_name(p) else {
                return false;
            };
            name.contains("Launchpad")
        })
    });

    if lp_in.is_none() {
        println!("No valid Launchpad found in {} input ports (waiting for device to initialize...)", in_ports.len());
    }
    if lp_out.is_none() {
        println!("No valid Launchpad found in {} output ports (waiting for device to initialize...)", out_ports.len());
    }

    if let (Some(in_port), Some(out_port)) = (lp_in, lp_out) {
        let in_name = midi_in.port_name(in_port).unwrap_or_else(|_| "Unknown".to_string());
        let out_name = midi_out.port_name(out_port).unwrap_or_else(|_| "Unknown".to_string());
        println!("Selected Launchpad Input: {}", in_name);
        println!("Selected Launchpad Output: {}", out_name);

        let tx = tx_event.clone();

        let _conn_in = midi_in.connect(
            in_port,
            "launchpad-in",
            move |_stamp, message, _| {
                if message.len() >= 3 {
                    let status = message[0] & 0xF0;
                    match status {
                        0x90 => {
                            let note = message[1];
                            let vel = message[2];
                            if vel > 0 {
                                let _ = tx.send(MidiEvent::NoteOn { note, velocity: vel });
                            }
                        }
                        0xB0 => {
                            let cc = message[1];
                            let val = message[2];
                            if val > 0 {
                                let _ = tx.send(MidiEvent::ControlChange {
                                    controller: cc,
                                    value: val,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            },
            (),
        )?;

        let mut conn_out = midi_out.connect(out_port, "launchpad-out")?;

        // Enter Programmer Mode
        // F0h 00h 20h 29h 02h 0Dh 0Eh 01h F7h
        conn_out.send(&[0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0x01, 0xF7])?;

        println!("Launchpad Programmer Mode Enabled");

        // Give Launchpad a moment to enter programmer mode
        thread::sleep(Duration::from_millis(100));

        // Now send connected event - Launchpad is ready for commands
        let _ = tx_event.send(MidiEvent::Connected);

        // Loop dealing with outgoing commands
        loop {
            let cmd = rx_cmd.recv()?;
            match cmd {
                MidiCommand::SetPadColor { note, color } => {
                    // Note On Ch 1 (0x90)
                    conn_out.send(&[0x90, note, color])?;
                }
                MidiCommand::SetButtonColor { cc, color } => {
                    // CC Ch 1 (0xB0)
                    conn_out.send(&[0xB0, cc, color])?;
                }
                MidiCommand::ClearAll => {
                    // Clear all Notes and CCs
                    // Launchpad mini mk3 covers roughly 0-99 space effectively
                    for i in 0..127 {
                         // Note Off
                         conn_out.send(&[0x90, i, 0])?;
                         // CC Off
                         conn_out.send(&[0xB0, i, 0])?;
                    }
                }
            }
        }
    } else {
        return Err("Launchpad not found".into());
    }
}
