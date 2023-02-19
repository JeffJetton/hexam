/*
    hexamine

    January 2023 - Jeff Jetton <jeff@jeffjetton.com>

    Displays hex and ASCII from a binary file.  Similar to hexdump and xxd,
    but designed for working on Atari 2600 cartridges, Apple 1/II programs,
    and other files that run on 8-bit systems.

    The address displayed on the left is masked to 16 bits, regardless of
    actual value.  Default starting address is 0x0000, but user can set it to
    a different origin or have the origin pulled from first two bytes of file
    (corresponding to dasm's default -f1 output format).

    Wozmon format omits the ASCII display and formats the hex with only
    eight bytes per line, no left-padding, and some slight spacing changes.

*/

use clap::Parser;
use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;


const STD_BYTES_PER_LINE: usize = 16;
const STD_BYTES_PER_SEGMENT: usize = 8;
const WOZ_BYTES_PER_LINE: usize = 8;
const WOZ_BYTES_PER_SEGMENT: usize = 0;  // 0 = "no segmenting"


/// Display a file in hexadecimal and ASCII
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// File to display
    file: std::path::PathBuf,
    /// Override default zero origin (pass 0 to get origin from first two bytes of <FILE> in little-endian order)
    #[arg(short, long, value_name="ADDRESS")]
    origin: Option<String>,
    /// Display in wozmon format
    #[arg(short, long)]
    woz: bool,
}

#[derive(Debug)]
struct LineFormat {
    bytes_per_line: usize,
    bytes_per_segment: usize,
    left_padding: usize,
    show_ascii: bool,
}


// Attempt to parse string as 16-bit hex value
fn parse_hex(s: &str) -> Result<usize, std::num::ParseIntError> {
    let hex = match usize::from_str_radix(s, 16) {
        Ok(x) => x,
        Err(error) => {eprintln!("Invalid hexadecimal value \"{s}\": {error}"); return Err(error);}
    };
    Ok(hex)
}


// Print one line of bytes (buffer may be shorter than full BYTES_PER_LINE)
fn print_buffer(bytes: &Vec<u8>, line_addr: usize, fmt: &LineFormat) {
    let mut line = String::with_capacity(80);

    // Start each line with the address of first byte (masked to 16 bits)
    write!(line, "{:04X}:", line_addr & 0xFFFF).unwrap();
    
    // Cycle through each position for the hex part of the line
    let mut i = 0;  // Current index into bytes vector
    for pos in 0..fmt.bytes_per_line {
        // Extra space at segment boundaries
        if (fmt.bytes_per_segment > 0) && (pos % fmt.bytes_per_segment) == 0 {
            line.push(' ')
        };
        // Pad (left or right) if needed
        if (pos < fmt.left_padding) || (fmt.show_ascii && i >= bytes.len()) {
            line.push_str("   ");
        } else {
            // Otherwise, show byte in hex if we have any bytes left
            if i < bytes.len() {
                write!(line, " {:02X}", bytes[i]).unwrap();
                i += 1;
            }
        }
    }

    if fmt.show_ascii {
        // A bit of space between hex and characters...
        let mut pad_remaining = fmt.left_padding + 2;
        while pad_remaining > 0 {
            line.push(' ');
            pad_remaining -= 1;
        }
        // Show characters
        for byte in bytes {
            line.push(if *byte >= 0x20_u8 && *byte < 0x7F {
                          *byte as char
                      } else {
                          '.'
                      });
        }
    }

    println!("{}", line);
}



fn main() -> std::io::Result<()> {
    
    let args = Cli::parse();
    let mut addr: usize = 0;

    // Get file and open a buffered reader on it
    let file = File::open(args.file)?;  // TODO: Better error message here?
    let mut buf_reader = BufReader::new(file);
    
    // Update starting address if user specified an origin
    if let Some(origin) = args.origin.as_deref() {
        addr = match parse_hex(origin) {
            Ok(x) => x,
            Err(_) => {return Ok(())},
        };
        // Use first two bytes as origin if user specified zero
        if addr == 0 {
            let mut origin_buffer = [0; 2];
            buf_reader.read_exact(&mut origin_buffer)?; // TODO: Better error message here?
            addr = (origin_buffer[1] as usize) * 0x100 + origin_buffer[0] as usize;
        }
    }

    // Initialize formatting parameters
    let mut fmt = if args.woz {
        LineFormat {bytes_per_line: WOZ_BYTES_PER_LINE,
                    bytes_per_segment: WOZ_BYTES_PER_SEGMENT,
                    left_padding: 0,
                    show_ascii: false,
                   }
    } else {
        LineFormat {bytes_per_line: STD_BYTES_PER_LINE,
                    bytes_per_segment: STD_BYTES_PER_SEGMENT,
                    left_padding: addr % STD_BYTES_PER_LINE,
                    show_ascii: true,
                   }
    };
    
    // Set up buffers, etc.
    let mut byte_buffer = [0];
    let mut line_buffer = Vec::with_capacity(fmt.bytes_per_line);
    let mut bytes_read: usize;
    let mut line_addr = addr;
    
    // MAIN LOOP: Read byte-by-byte and print when we have enough for a line
    loop {
        bytes_read = buf_reader.read(&mut byte_buffer)?;  // TODO: Better error
        if bytes_read == 0 {
            // End of file... print out line buffer if it's got anything in it
            if !line_buffer.is_empty() {
                print_buffer(&line_buffer, line_addr, &fmt);
            }
            break;
        } else {
            line_buffer.push(byte_buffer[0]);
            addr += 1;
            if addr % fmt.bytes_per_line == 0 {
                // Print buffer
                print_buffer(&line_buffer, line_addr, &fmt);
                // Clear the buffer and any initial padding
                line_buffer.clear();
                fmt.left_padding = 0;
                // Remember starting address of new line
                line_addr = addr;
            }
        }
    }

    Ok(())
}
