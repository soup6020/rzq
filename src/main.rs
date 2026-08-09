use std::env;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::thread;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";

#[derive(Clone, Copy)]
struct Palette {
    enabled: bool,
}

impl Palette {
    fn paint(self, style: &str, text: &str) -> String {
        if self.enabled {
            format!("{style}{text}{RESET}")
        } else {
            text.to_owned()
        }
    }

    fn heading(self, text: &str) -> String {
        self.paint(&format!("{BOLD}{CYAN}"), text)
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut color = env::var_os("NO_COLOR").is_none();

    while matches!(
        args.first().map(String::as_str),
        Some("--color" | "--no-color")
    ) {
        color = args.remove(0) == "--color";
    }

    let palette = Palette { enabled: color };
    match dispatch(&args, palette) {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(message) => {
            eprintln!(
                "{} {}",
                palette.paint(&format!("{BOLD}{RED}"), "error:"),
                message
            );
            eprintln!(
                "{}",
                palette.paint(DIM, "Run `rzq --help` for a command reference.")
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch(args: &[String], palette: Palette) -> Result<i32, String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help(palette);
        return Ok(0);
    };
    let rest = &args[1..];

    match command {
        "-h" | "--help" | "help" => {
            print_help(palette);
            Ok(0)
        }
        "-V" | "--version" | "version" => {
            println!(
                "{} {}",
                palette.heading("rzq"),
                palette.paint(GREEN, env!("CARGO_PKG_VERSION"))
            );
            Ok(0)
        }
        "info" | "i" => bin_command("info", &["-I"], rest, palette),
        "sections" | "sec" => sections(rest, palette),
        "headers" | "header" => bin_command("headers", &["-H"], rest, palette),
        "imports" | "imp" => bin_command("imports", &["-i"], rest, palette),
        "exports" | "exp" => bin_command("exports", &["-E"], rest, palette),
        "symbols" | "sym" => bin_command("symbols", &["-s"], rest, palette),
        "libs" | "libraries" => bin_command("libs", &["-l"], rest, palette),
        "entry" | "entrypoint" => bin_command("entry", &["-e"], rest, palette),
        "strings" | "str" => strings(rest, palette),
        "all" => bin_command("all", &["-g"], rest, palette),
        "hash" => hash(rest, palette),
        "hashes" => hashes(rest, palette),
        "b64" | "base64" => base64(rest, palette),
        "int" | "dec" => convert(rest, "=10", "int", palette),
        "hex" => convert(rest, "=16", "hex", palette),
        "num" | "number" => number(rest, palette),
        "hexdump" | "hd" | "xxd" => hexdump_command(rest, palette),
        "analyze" | "analyse" => analyze(rest, palette),
        _ => Err(format!("unknown command `{command}`")),
    }
}

fn one_file<'a>(name: &str, args: &'a [String]) -> Result<&'a str, String> {
    match args {
        [file] => Ok(file),
        [] => Err(format!("`{name}` requires a file")),
        _ => Err(format!("usage: rzq {name} FILE")),
    }
}

fn safe_file_arg(file: &str) -> String {
    if file.starts_with('-') && file != "-" && !file.contains('/') {
        format!("./{file}")
    } else {
        file.to_owned()
    }
}

fn bin_command(
    name: &str,
    flags: &[&str],
    args: &[String],
    palette: Palette,
) -> Result<i32, String> {
    let file = safe_file_arg(one_file(name, args)?);
    let mut tool_args: Vec<String> = flags.iter().map(|flag| (*flag).to_owned()).collect();
    tool_args.push(file);
    run_tool("rz-bin", &tool_args, palette.enabled)
}

fn sections(args: &[String], palette: Palette) -> Result<i32, String> {
    let mut algorithm = "md5";
    let mut file = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-K" | "--hash" => {
                index += 1;
                algorithm = args.get(index).ok_or("`--hash` requires an algorithm")?;
            }
            value if value.starts_with("--hash=") => algorithm = &value[7..],
            value if file.is_none() => file = Some(value),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        index += 1;
    }
    let file = safe_file_arg(file.ok_or("`sections` requires a file")?);
    run_tool(
        "rz-bin",
        &["-K".into(), algorithm.into(), "-S".into(), file],
        palette.enabled,
    )
}

fn strings(args: &[String], palette: Palette) -> Result<i32, String> {
    let mut raw = false;
    let mut file = None;
    for arg in args {
        match arg.as_str() {
            "-a" | "--all" | "--raw" => raw = true,
            value if file.is_none() => file = Some(value),
            value => return Err(format!("unexpected argument `{value}`")),
        }
    }
    let file = safe_file_arg(file.ok_or("`strings` requires a file")?);
    run_tool(
        "rz-bin",
        &[if raw { "-zz" } else { "-z" }.into(), file],
        palette.enabled,
    )
}

fn hash(args: &[String], palette: Palette) -> Result<i32, String> {
    let Some(algorithm) = args.first() else {
        return Err("usage: rzq hash ALGORITHM [FILE ...]".into());
    };
    let mut tool_args = vec!["-a".into(), algorithm.clone()];
    if args.len() == 1 {
        tool_args.push("-".into());
    } else {
        tool_args.extend(args[1..].iter().map(|file| safe_file_arg(file)));
    }
    run_tool("rz-hash", &tool_args, palette.enabled)
}

fn hashes(args: &[String], palette: Palette) -> Result<i32, String> {
    let file = safe_file_arg(one_file("hashes", args)?);
    run_tool(
        "rz-hash",
        &["-a".into(), "md5,sha1,sha256".into(), file],
        palette.enabled,
    )
}

fn base64(args: &[String], palette: Palette) -> Result<i32, String> {
    let Some(operation) = args.first().map(String::as_str) else {
        return Err("usage: rzq b64 <encode|decode> [TEXT]".into());
    };
    let flag = match operation {
        "encode" | "enc" | "e" => "-E",
        "decode" | "dec" | "d" => "-D",
        _ => return Err(format!("unknown base64 operation `{operation}`")),
    };
    if args.len() > 2 {
        return Err("base64 text containing spaces must be quoted".into());
    }
    let mut tool_args = vec!["-l".into(), flag.into()];
    if let Some(text) = args.get(1) {
        tool_args.push(text.clone());
    }
    run_tool("rz-ax", &tool_args, palette.enabled)
}

fn convert(args: &[String], base: &str, name: &str, palette: Palette) -> Result<i32, String> {
    let expression = expression_arg(args, name)?;
    run_tool("rz-ax", &[base.to_owned(), expression], palette.enabled)
}

fn number(args: &[String], palette: Palette) -> Result<i32, String> {
    let expression = expression_arg(args, "num")?;
    print_section("decimal", palette)?;
    let code = run_tool(
        "rz-ax",
        &["=10".into(), expression.clone()],
        palette.enabled,
    )?;
    if code != 0 {
        return Ok(code);
    }
    print_section("hexadecimal", palette)?;
    run_tool("rz-ax", &["=16".into(), expression], palette.enabled)
}

fn expression_arg(args: &[String], name: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err(format!("`{name}` requires a number or expression"));
    }
    Ok(args
        .join("")
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect())
}

fn analyze(args: &[String], palette: Palette) -> Result<i32, String> {
    let file = safe_file_arg(one_file("analyze", args)?);
    let jobs = [
        (
            "binary information",
            "rz-bin",
            vec!["-I".into(), file.clone()],
        ),
        (
            "section MD5 checksums",
            "rz-bin",
            vec!["-K".into(), "md5".into(), "-S".into(), file.clone()],
        ),
        (
            "file hashes",
            "rz-hash",
            vec!["-a".into(), "md5,sha1,sha256".into(), file],
        ),
    ];
    for (heading, tool, tool_args) in jobs {
        print_section(heading, palette)?;
        let code = run_tool(tool, &tool_args, palette.enabled)?;
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}

fn print_section(text: &str, palette: Palette) -> Result<(), String> {
    let mut out = io::stdout().lock();
    writeln!(out, "\n{}", palette.heading(&format!("── {text} ──"))).map_err(io_error)
}

fn run_tool(tool: &str, args: &[String], color: bool) -> Result<i32, String> {
    if !color {
        return Command::new(tool)
            .args(args)
            .env("RZ_COLOR", "0")
            .status()
            .map(|status| status.code().unwrap_or(1))
            .map_err(|error| tool_error(tool, error));
    }

    let mut child = Command::new(tool)
        .args(args)
        .env("RZ_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| tool_error(tool, error))?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out_thread = thread::spawn(move || color_copy(stdout, io::stdout(), CYAN));
    let err_thread = thread::spawn(move || color_copy(stderr, io::stderr(), RED));
    let status = child.wait().map_err(|error| tool_error(tool, error))?;
    out_thread
        .join()
        .map_err(|_| "stdout worker panicked".to_owned())?
        .map_err(io_error)?;
    err_thread
        .join()
        .map_err(|_| "stderr worker panicked".to_owned())?
        .map_err(io_error)?;
    Ok(status.code().unwrap_or(1))
}

fn color_copy<R: Read, W: Write>(mut reader: R, mut writer: W, fallback: &str) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    let mut pending = Vec::with_capacity(buffer.len() + RESET.len());
    let mut started = false;

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if !started {
            writer.write_all(fallback.as_bytes())?;
            started = true;
        }

        pending.extend_from_slice(&buffer[..count]);
        let keep = incomplete_reset_suffix(&pending);
        let ready = pending.len() - keep;
        write_with_fallback(&mut writer, &pending[..ready], fallback)?;
        pending = pending.split_off(ready);
    }

    if !started {
        return Ok(());
    }
    writer.write_all(&pending)?;
    writer.write_all(RESET.as_bytes())?;
    writer.flush()
}

fn incomplete_reset_suffix(bytes: &[u8]) -> usize {
    let reset = RESET.as_bytes();
    (1..reset.len())
        .rev()
        .find(|&length| bytes.ends_with(&reset[..length]))
        .unwrap_or(0)
}

fn write_with_fallback<W: Write>(writer: &mut W, bytes: &[u8], fallback: &str) -> io::Result<()> {
    let reset = RESET.as_bytes();
    let mut start = 0;
    while let Some(position) = bytes[start..]
        .windows(reset.len())
        .position(|window| window == reset)
    {
        let end = start + position + reset.len();
        writer.write_all(&bytes[start..end])?;
        writer.write_all(fallback.as_bytes())?;
        start = end;
    }
    writer.write_all(&bytes[start..])
}

fn tool_error(tool: &str, error: io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        format!("could not find `{tool}`; install Rizin or use the Nix package")
    } else {
        format!("failed to run `{tool}`: {error}")
    }
}

fn hexdump_command(args: &[String], palette: Palette) -> Result<i32, String> {
    let mut file = None;
    let mut bytes = 256_u64;
    let mut offset = 0_u64;
    let mut width = 16_usize;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-n" | "--bytes" => {
                index += 1;
                bytes = parse_limit(args.get(index).ok_or("`--bytes` requires a value")?)?;
            }
            "-s" | "--offset" => {
                index += 1;
                offset = parse_size(args.get(index).ok_or("`--offset` requires a value")?)?;
            }
            "-w" | "--width" => {
                index += 1;
                width = parse_size(args.get(index).ok_or("`--width` requires a value")?)? as usize;
            }
            value if value.starts_with("--bytes=") => bytes = parse_limit(&value[8..])?,
            value if value.starts_with("--offset=") => offset = parse_size(&value[9..])?,
            value if value.starts_with("--width=") => width = parse_size(&value[8..])? as usize,
            value if file.is_none() => file = Some(value),
            value if bytes == 256 => bytes = parse_limit(value)?,
            value => return Err(format!("unexpected argument `{value}`")),
        }
        index += 1;
    }

    if !(1..=64).contains(&width) {
        return Err("hexdump width must be between 1 and 64".into());
    }
    let file = file.ok_or("usage: rzq hexdump FILE [--bytes N] [--offset N]")?;
    let result = if file == "-" {
        let mut input = io::stdin().lock();
        discard(&mut input, offset)?;
        write_hexdump(&mut input, offset, bytes, width, palette)
    } else {
        let mut input = File::open(Path::new(file))
            .map_err(|error| format!("cannot open `{file}`: {error}"))?;
        input.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        write_hexdump(&mut input, offset, bytes, width, palette)
    };

    match result {
        Err(message) if message.contains("Broken pipe") => Ok(0),
        Err(message) => Err(message),
        Ok(()) => Ok(0),
    }
}

fn parse_limit(value: &str) -> Result<u64, String> {
    if matches!(value, "all" | "ALL" | "*") {
        Ok(u64::MAX)
    } else {
        parse_size(value)
    }
}

fn parse_size(value: &str) -> Result<u64, String> {
    let cleaned = value.replace('_', "");
    let (number, multiplier) = match cleaned.as_bytes().last().copied() {
        Some(b'k' | b'K') => (&cleaned[..cleaned.len() - 1], 1024_u64),
        Some(b'm' | b'M') => (&cleaned[..cleaned.len() - 1], 1024_u64.pow(2)),
        Some(b'g' | b'G') => (&cleaned[..cleaned.len() - 1], 1024_u64.pow(3)),
        _ => (cleaned.as_str(), 1),
    };
    let parsed = if let Some(hex) = number
        .strip_prefix("0x")
        .or_else(|| number.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        number.parse()
    }
    .map_err(|_| format!("invalid size `{value}`"))?;
    parsed
        .checked_mul(multiplier)
        .ok_or_else(|| format!("size `{value}` is too large"))
}

fn discard<R: Read>(reader: &mut R, mut count: u64) -> Result<(), String> {
    let mut buffer = [0_u8; 8192];
    while count > 0 {
        let wanted = usize::try_from(count.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader.read(&mut buffer[..wanted]).map_err(io_error)?;
        if read == 0 {
            break;
        }
        count -= read as u64;
    }
    Ok(())
}

fn write_hexdump<R: Read>(
    reader: &mut R,
    mut address: u64,
    mut remaining: u64,
    width: usize,
    palette: Palette,
) -> Result<(), String> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    let mut row = vec![0_u8; width];

    while remaining > 0 {
        let wanted = usize::try_from(remaining.min(width as u64)).unwrap_or(width);
        let mut used = 0;
        while used < wanted {
            let count = reader.read(&mut row[used..wanted]).map_err(io_error)?;
            if count == 0 {
                break;
            }
            used += count;
        }
        if used == 0 {
            break;
        }

        write!(
            output,
            "{}  ",
            palette.paint(&format!("{BOLD}{BLUE}"), &format!("{address:08x}"))
        )
        .map_err(io_error)?;
        for (index, byte) in row[..used].iter().enumerate() {
            let style = if *byte == 0 {
                DIM
            } else if byte.is_ascii_graphic() {
                GREEN
            } else {
                YELLOW
            };
            write!(output, "{}", palette.paint(style, &format!("{byte:02x}"))).map_err(io_error)?;
            if index + 1 != width {
                write!(output, " ").map_err(io_error)?;
            }
            if index + 1 == width / 2 {
                write!(output, " ").map_err(io_error)?;
            }
        }
        if used < width {
            for index in used..width {
                write!(output, "  ").map_err(io_error)?;
                if index + 1 != width {
                    write!(output, " ").map_err(io_error)?;
                }
                if index + 1 == width / 2 {
                    write!(output, " ").map_err(io_error)?;
                }
            }
        }
        write!(output, "  {}", palette.paint(DIM, "│")).map_err(io_error)?;
        for byte in &row[..used] {
            let character = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '·'
            };
            write!(output, "{}", palette.paint(MAGENTA, &character.to_string()))
                .map_err(io_error)?;
        }
        writeln!(output, "{}", palette.paint(DIM, "│")).map_err(io_error)?;

        address = address.saturating_add(used as u64);
        remaining -= used as u64;
    }
    output.flush().map_err(io_error)
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

fn print_help(p: Palette) {
    let title = p.paint(&format!("{BOLD}{CYAN}"), "rzq");
    let accent = |text: &str| p.paint(GREEN, text);
    let arg = |text: &str| p.paint(YELLOW, text);
    let dim = |text: &str| p.paint(DIM, text);

    println!("{title} — quick, colourful Rizin file analysis\n");
    println!(
        "{} {} {}",
        p.heading("Usage:"),
        accent("rzq"),
        arg("[--color|--no-color] COMMAND [ARGS]")
    );
    println!();
    println!("{}", p.heading("Quick analysis"));
    help_line(
        &accent("analyze FILE"),
        "Info, section MD5s, and common file hashes",
    );
    help_line(&accent("info FILE"), "Binary information (rz-bin -I)");
    help_line(
        &accent("sections FILE [--hash ALG]"),
        "Sections with checksums; md5 by default",
    );
    help_line(
        &accent("hash ALG [FILE ...]"),
        "Hash files or stdin; ALG may be md5,sha256,...",
    );
    help_line(
        &accent("hashes FILE"),
        "MD5, SHA-1, and SHA-256 in one pass",
    );
    help_line(
        &accent("hexdump FILE [N]"),
        "Colour hexdump; N defaults to 256 bytes",
    );
    println!();
    println!("{}", p.heading("Binary details"));
    help_line(&accent("headers FILE"), "Header fields");
    help_line(&accent("imports FILE"), "Imported symbols");
    help_line(&accent("exports FILE"), "Globally exported symbols");
    help_line(&accent("symbols FILE"), "Symbols");
    help_line(&accent("libs FILE"), "Linked libraries");
    help_line(&accent("entry FILE"), "Entrypoint");
    help_line(
        &accent("strings FILE [--all]"),
        "Data strings, or raw-file strings",
    );
    help_line(&accent("all FILE"), "All rz-bin information (rz-bin -g)");
    println!();
    println!("{}", p.heading("Data and numbers"));
    help_line(
        &accent("b64 encode [TEXT]"),
        "Base64 encode text or stdin (rz-ax -E)",
    );
    help_line(
        &accent("b64 decode [TEXT]"),
        "Base64 decode text or stdin (rz-ax -D)",
    );
    help_line(&accent("int EXPR"), "Evaluate and print EXPR in decimal");
    help_line(
        &accent("hex EXPR"),
        "Evaluate and print EXPR in hexadecimal",
    );
    help_line(&accent("num EXPR"), "Print both decimal and hexadecimal");
    println!();
    println!("{}", p.heading("Hexdump options"));
    help_line(
        &arg("-n, --bytes N"),
        "Bytes to print: decimal, 0xHEX, K/M/G, or all",
    );
    help_line(&arg("-s, --offset N"), "Start offset (same number formats)");
    help_line(&arg("-w, --width N"), "Bytes per row (1–64; default 16)");
    help_line(&dim("FILE may be -"), "Read from standard input");
    println!();
    println!("{}", p.heading("Examples"));
    println!("  {}", accent("rzq analyze ./program"));
    println!("  {}", accent("rzq sections --hash sha256 firmware.bin"));
    println!("  {}", accent("rzq hash md5 disk.img"));
    println!("  {}", accent("printf hello | rzq b64 encode"));
    println!("  {}", accent("rzq int '0x40 + 6'     # 70"));
    println!("  {}", accent("rzq hex 255            # 0xff"));
    println!("  {}", accent("rzq hd firmware.bin -s 0x100 -n 1K"));
    println!();
    println!(
        "{}",
        dim("Aliases: i, sec, imp, exp, sym, str, hd, xxd, dec, base64")
    );
    println!(
        "{}",
        dim("Colour is enabled by default. NO_COLOR and --no-color disable it.")
    );
}

fn help_line(command: &str, description: &str) {
    println!("  {command:<38} {description}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("16").unwrap(), 16);
        assert_eq!(parse_size("0x100").unwrap(), 256);
        assert_eq!(parse_size("2K").unwrap(), 2048);
        assert_eq!(parse_size("1_M").unwrap(), 1024 * 1024);
        assert!(parse_size("wat").is_err());
    }

    #[test]
    fn parses_unlimited_byte_count() {
        assert_eq!(parse_limit("all").unwrap(), u64::MAX);
        assert_eq!(parse_limit("*").unwrap(), u64::MAX);
    }

    #[test]
    fn protects_dash_prefixed_files() {
        assert_eq!(safe_file_arg("-sample"), "./-sample");
        assert_eq!(safe_file_arg("-"), "-");
        assert_eq!(safe_file_arg("tmp/-sample"), "tmp/-sample");
    }

    #[test]
    fn preserves_native_colors_and_restores_the_fallback() {
        let input = format!("plain{GREEN}native{RESET}plain");
        let mut output = Vec::new();
        color_copy(input.as_bytes(), &mut output, CYAN).unwrap();
        assert_eq!(
            output,
            format!("{CYAN}plain{GREEN}native{RESET}{CYAN}plain{RESET}").as_bytes()
        );
    }
}
