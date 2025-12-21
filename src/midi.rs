use midir::{Ignore, MidiInput, MidiOutput};
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    ControlChange { controller: u8, value: u8 },
}

pub enum MidiCommand {
    SetPadColor { note: u8, color: u8 },
    SetButtonColor { cc: u8, color: u8 },
    ClearAll,
}

pub fn start_midi_service(tx_to_app: Sender<MidiEvent>) -> Sender<MidiCommand> {
    let (tx_cmd, rx_cmd) = std::sync::mpsc::channel();

    thread::spawn(move || loop {
        match run_midi_loop(&tx_to_app, &rx_cmd) {
            Ok(_) => break,
            Err(e) => {
                eprintln!("MIDI Error: {:?}. Retrying in 5s...", e);
                thread::sleep(Duration::from_secs(5));
            }
        }
    });

    tx_cmd
}

fn run_midi_loop(
    tx_event: &Sender<MidiEvent>,
    rx_cmd: &Receiver<MidiCommand>,
) -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("Lightspeed Input")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("Lightspeed Output")?;

    let in_ports = midi_in.ports();
    let out_ports = midi_out.ports();

    // List all ports for debugging
    println!("\nAvailable Input Ports:");
    for (i, p) in in_ports.iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p).unwrap_or_default());
    }
    println!("\nAvailable Output Ports:");
    for (i, p) in out_ports.iter().enumerate() {
        println!("{}: {}", i, midi_out.port_name(p).unwrap_or_default());
    }

    // Smart Selection Logic
    // 1. Prefer "Launchpad" AND "MIDI"
    // 2. Prefer "Launchpad" AND NOT "DAW"
    // 3. Fallback to any "Launchpad"
    
    // Find Input
    let lp_in = in_ports.iter().find(|p| {
        let name = midi_in.port_name(p).unwrap_or_default();
        name.contains("Launchpad") && (name.contains("MIDI") || name.contains("LPMiniMK3 MIDI"))
    }).or_else(|| {
        in_ports.iter().find(|p| {
            let name = midi_in.port_name(p).unwrap_or_default();
            name.contains("Launchpad") && !name.contains("DAW")
        })
    }).or_else(|| {
        in_ports.iter().find(|p| {
            midi_in.port_name(p).unwrap_or_default().contains("Launchpad")
        })
    });

    // Find Output
    let lp_out = out_ports.iter().find(|p| {
        let name = midi_out.port_name(p).unwrap_or_default();
        name.contains("Launchpad") && (name.contains("MIDI") || name.contains("LPMiniMK3 MIDI"))
    }).or_else(|| {
        out_ports.iter().find(|p| {
            let name = midi_out.port_name(p).unwrap_or_default();
            name.contains("Launchpad") && !name.contains("DAW")
        })
    }).or_else(|| {
        out_ports.iter().find(|p| {
            midi_out.port_name(p).unwrap_or_default().contains("Launchpad")
        })
    });

    if let (Some(in_port), Some(out_port)) = (lp_in, lp_out) {
        println!("Selected Launchpad Input: {}", midi_in.port_name(in_port)?);
        println!("Selected Launchpad Output: {}", midi_out.port_name(out_port)?);
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
