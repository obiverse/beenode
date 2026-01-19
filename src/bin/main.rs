//! Beenode CLI - Scroll I/O Interface
//!
//! All operations are scroll I/O:
//!   beenode get <path>           → Read scroll, output JSON
//!   beenode put <path> <json>    → Write scroll, output result
//!   beenode list [prefix]        → List paths, output JSON array
//!
//! Wallet operations are just paths:
//!   beenode get /wallet/balance  → {"confirmed": 0, "pending": 0, "total": 0}
//!   beenode get /wallet/address  → {"address": "bc1q..."}
//!   beenode put /wallet/sync {}  → Queue sync effect
//!
//! Configuration:
//!   beenode init --app <name> --mnemonic <words> --network <net> --electrum <url>
//!
//! Output format:
//!   --json     Output raw JSON (default for non-tty)
//!   --pretty   Pretty-print JSON (default for tty)
//!   --scroll   Output full scroll (key, type, metadata, data)

use beenode::{AuthMode, Node, NodeConfig};
use beenode::auth::PinAuth;
use beenode::logging::init_logging;
use serde_json::{json, Value};
use std::env;
use std::io::{self, IsTerminal, Write};
use tracing::{debug, info};

#[cfg(feature = "wallet")]
use beenode::{Network, WalletConfig};

#[cfg(feature = "nostr")]
use beenode::node::NostrConfig;

fn main() {
    init_logging();
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = env::args().collect();
    let opts = ParsedArgs::parse(&args[1..]);

    if opts.help {
        print_usage();
        return;
    }

    if opts.version {
        println!("beenode 0.1.0");
        return;
    }

    let result = match opts.command.as_deref() {
        Some("init") => cmd_init(&opts),
        Some("get") => cmd_get(&opts),
        Some("put") => cmd_put(&opts),
        Some("list") | Some("ls") => cmd_list(&opts),
        Some("repl") => cmd_repl(&opts),
        Some("serve") => cmd_serve(&opts),
        Some(cmd) => Err(format!("Unknown command: {}", cmd)),
        None => {
            print_usage();
            return;
        }
    };

    match result {
        Ok(output) => {
            let formatted = if opts.scroll {
                serde_json::to_string_pretty(&output).unwrap()
            } else if opts.pretty || std::io::stdout().is_terminal() {
                // Extract just data if it's a scroll
                if let Some(data) = output.get("data") {
                    serde_json::to_string_pretty(data).unwrap()
                } else {
                    serde_json::to_string_pretty(&output).unwrap()
                }
            } else {
                if let Some(data) = output.get("data") {
                    serde_json::to_string(data).unwrap()
                } else {
                    serde_json::to_string(&output).unwrap()
                }
            };
            println!("{}", formatted);
        }
        Err(e) => {
            let err = json!({"error": e});
            if opts.pretty || std::io::stdout().is_terminal() {
                eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
            } else {
                eprintln!("{}", serde_json::to_string(&err).unwrap());
            }
            std::process::exit(1);
        }
    }
}

#[derive(Default)]
struct ParsedArgs {
    command: Option<String>,
    path: Option<String>,
    data: Option<String>,
    // Init options
    app: Option<String>,
    mnemonic: Option<String>,
    network: Option<String>,
    electrum_url: Option<String>,
    relays: Vec<String>,
    data_dir: Option<String>,
    pin: Option<String>,
    auth_mode: Option<String>,
    // RPC options (for bitcoind-rpc feature)
    rpc_url: Option<String>,
    rpc_user: Option<String>,
    rpc_pass: Option<String>,
    // Server options
    port: Option<u16>,
    // Output options
    json: bool,
    pretty: bool,
    scroll: bool,
    help: bool,
    version: bool,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Self {
        // Load .env file if present
        if let Ok(contents) = std::fs::read_to_string(".env") {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let value = value.trim().trim_matches('"');
                    if !value.is_empty() && env::var(key.trim()).is_err() {
                        env::set_var(key.trim(), value);
                    }
                }
            }
        }

        let mut opts = ParsedArgs::default();
        let mut positional = Vec::new();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];
            match arg.as_str() {
                "--help" | "-h" => opts.help = true,
                "--version" | "-V" => opts.version = true,
                "--json" => opts.json = true,
                "--pretty" => opts.pretty = true,
                "--scroll" => opts.scroll = true,
                "--app" | "-a" => {
                    if i + 1 < args.len() {
                        opts.app = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--mnemonic" | "-m" => {
                    if i + 1 < args.len() {
                        opts.mnemonic = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--network" | "-n" => {
                    if i + 1 < args.len() {
                        opts.network = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--electrum" | "-e" => {
                    if i + 1 < args.len() {
                        opts.electrum_url = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--relay" | "-r" => {
                    if i + 1 < args.len() {
                        opts.relays.push(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--data-dir" | "-d" => {
                    if i + 1 < args.len() {
                        opts.data_dir = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--pin" => {
                    if i + 1 < args.len() {
                        opts.pin = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--auth" | "--auth-mode" => {
                    if i + 1 < args.len() {
                        opts.auth_mode = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--port" | "-p" => {
                    if i + 1 < args.len() {
                        opts.port = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                _ if !arg.starts_with('-') => positional.push(arg.clone()),
                _ => {} // Ignore unknown flags
            }
            i += 1;
        }

        // First positional is command
        if !positional.is_empty() {
            opts.command = Some(positional.remove(0));
        }
        // Second positional is path
        if !positional.is_empty() {
            opts.path = Some(positional.remove(0));
        }
        // Rest is data (joined)
        if !positional.is_empty() {
            opts.data = Some(positional.join(" "));
        }

        // Apply environment variables (lower priority than CLI args)
        if opts.app.is_none() {
            opts.app = env::var("BEENODE_APP").ok();
        }
        if opts.mnemonic.is_none() {
            opts.mnemonic = env::var("BEENODE_MNEMONIC").ok();
        }
        if opts.network.is_none() {
            opts.network = env::var("BEENODE_NETWORK").ok();
        }
        if opts.electrum_url.is_none() {
            opts.electrum_url = env::var("BEENODE_ELECTRUM").ok().filter(|s| !s.is_empty());
        }
        if opts.data_dir.is_none() {
            opts.data_dir = env::var("BEENODE_DATA_DIR").ok().filter(|s| !s.is_empty());
        }
        if opts.auth_mode.is_none() {
            opts.auth_mode = env::var("BEENODE_AUTH_MODE").ok().filter(|s| !s.is_empty());
        }
        if opts.relays.is_empty() {
            if let Ok(relays) = env::var("BEENODE_RELAYS") {
                opts.relays = relays.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
        }

        // RPC options from BITCOIN_RPC_* env vars
        if opts.rpc_url.is_none() {
            opts.rpc_url = env::var("BITCOIN_RPC_URL").ok().filter(|s| !s.is_empty());
        }
        if opts.rpc_user.is_none() {
            opts.rpc_user = env::var("BITCOIN_RPC_USER").ok().filter(|s| !s.is_empty());
        }
        if opts.rpc_pass.is_none() {
            opts.rpc_pass = env::var("BITCOIN_RPC_PASS").ok().filter(|s| !s.is_empty());
        }

        // Server port from env
        if opts.port.is_none() {
            opts.port = env::var("BEENODE_PORT").ok().and_then(|s| s.parse().ok());
        }

        opts
    }
}

fn print_usage() {
    println!(
        r#"beenode - Scroll I/O Interface

USAGE:
    beenode <command> [path] [data] [options]

COMMANDS:
    init                    Initialize node (creates config)
    get <path>              Read scroll at path
    put <path> <json>       Write scroll to path
    list [prefix]           List paths under prefix
    repl                    Interactive mode
    serve                   Start HTTP server

SERVER OPTIONS:
    --port, -p <port>       Server port (default: 8080, env: BEENODE_PORT)

INIT OPTIONS:
    --app, -a <name>        Application name (required)
    --mnemonic, -m <words>  BIP39 mnemonic (12/24 words)
    --network, -n <net>     Network: bitcoin|testnet|signet|regtest
    --electrum, -e <url>    Electrum server URL
    --relay, -r <url>       Nostr relay URL (can repeat)
    --data-dir, -d <path>   Data directory
    --pin <pin>             Unlock PIN for operations
    --auth <mode>           Auth mode: pin|none (env: BEENODE_AUTH_MODE)

OUTPUT OPTIONS:
    --json                  Raw JSON output
    --pretty                Pretty-print JSON
    --scroll                Output full scroll (key, type, metadata, data)
    --version, -V           Print version

SCROLL PATHS:
    /wallet/status          → {{initialized, network}}
    /wallet/balance         → {{confirmed, pending, total}}
    /wallet/address         → {{address}}
    /wallet/transactions    → {{transactions, count}}
    /wallet/sync            ← {{}} (write to sync)
    /wallet/send            ← {{to, amount_sat}} (write to send)

    /nostr/status           → {{initialized, relays}}
    /nostr/pubkey           → {{hex}}
    /nostr/mobi             → {{display, formatted, full}}
    /nostr/sign             ← {{message}} (write to sign)

    /system/auth/status      → {{locked, initialized}}
    /system/auth/unlock      ← {{pin}} (unlock with PIN)
    /system/auth/lock        ← {{}} (lock node)

EXAMPLES:
    # Initialize
    beenode init --app myapp --mnemonic "abandon ... about" --network regtest

    # Read wallet
    beenode get /wallet/balance
    beenode get /wallet/address --scroll

    # Write to wallet
    beenode put /wallet/sync '{{}}'
    beenode put /wallet/send '{{"to":"bc1q...","amount_sat":10000}}'

    # List paths
    beenode list /wallet

    # Pipe-friendly
    beenode get /wallet/balance --json | jq .confirmed
"#
    );
}

fn config_path(app: &str) -> String {
    format!(".beenode-{}.json", app)
}

fn save_config(app: &str, opts: &ParsedArgs, auth_mode: AuthMode, mnemonic: Option<&str>) -> Result<(), String> {
    let mnemonic = if auth_mode == AuthMode::None { mnemonic } else { None };
    let config = json!({
        "app": app,
        "mnemonic": mnemonic,
        "auth_mode": auth_mode.as_str(),
        "network": opts.network.as_deref().unwrap_or("signet"),
        "electrum_url": opts.electrum_url,
        "relays": opts.relays,
        "data_dir": opts.data_dir,
        "rpc_url": opts.rpc_url,
        "rpc_user": opts.rpc_user,
        "rpc_pass": opts.rpc_pass,
    });
    let path = config_path(app);
    std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("Failed to save config: {}", e))?;
    Ok(())
}

fn load_config() -> Result<Value, String> {
    // Find config file in current directory
    let entries = std::fs::read_dir(".")
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(".beenode-") && name.ends_with(".json") {
            let data = std::fs::read_to_string(entry.path())
                .map_err(|e| format!("Failed to read config: {}", e))?;
            return serde_json::from_str(&data)
                .map_err(|e| format!("Invalid config JSON: {}", e));
        }
    }
    Err("No config found. Run 'beenode init --app <name>' first.".into())
}

fn parse_auth_mode(value: Option<&str>) -> Result<AuthMode, String> {
    let raw = value.unwrap_or("pin");
    AuthMode::from_str(raw)
        .ok_or_else(|| format!("Invalid auth mode: {}", raw))
}

fn load_node_from_env() -> Result<Node, String> {
    // All config from env (loaded from .env by ParsedArgs) with config fallback.
    let config = load_config().ok();
    let config_string = |key: &str| -> Option<String> {
        config
            .as_ref()
            .and_then(|cfg| cfg.get(key))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
    };

    let app = env::var("BEENODE_APP")
        .ok()
        .or_else(|| config_string("app"))
        .ok_or("BEENODE_APP not set")?;
    let auth_mode_raw = env::var("BEENODE_AUTH_MODE")
        .ok()
        .or_else(|| config_string("auth_mode"));
    let auth_mode = parse_auth_mode(auth_mode_raw.as_deref())?;
    let mut node_config = NodeConfig::new(&app).with_auth_mode(auth_mode);

    let auth_initialized = match auth_mode {
        AuthMode::Pin => PinAuth::load(&app)
            .map(|auth| auth.is_initialized())
            .unwrap_or(false),
        AuthMode::None => false,
    };
    if auth_mode == AuthMode::None || !auth_initialized {
        if let Some(m) = env::var("BEENODE_MNEMONIC").ok().or_else(|| config_string("mnemonic")) {
            node_config = node_config.with_mnemonic(&m);
        }
    }

    #[cfg(feature = "wallet")]
    {
        let network = env::var("BEENODE_NETWORK")
            .ok()
            .or_else(|| config_string("network"))
            .unwrap_or_else(|| "signet".into());
        let net = match network.as_str() {
            "bitcoin" | "mainnet" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Signet,
        };

        let electrum_url = env::var("BEENODE_ELECTRUM")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| config_string("electrum_url").filter(|s| !s.is_empty()));
        let data_dir = env::var("BEENODE_DATA_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| config_string("data_dir").filter(|s| !s.is_empty()))
            .map(std::path::PathBuf::from);

        let mut wallet_cfg = WalletConfig {
            network: net,
            electrum_url,
            data_dir,
            #[cfg(feature = "bitcoind-rpc")]
            rpc: None,
        };

        // Use RPC if configured (takes precedence over electrum)
        #[cfg(feature = "bitcoind-rpc")]
        if let (Some(url), Some(user), Some(pass)) = (
            env::var("BITCOIN_RPC_URL").ok().or_else(|| config_string("rpc_url")),
            env::var("BITCOIN_RPC_USER").ok().or_else(|| config_string("rpc_user")),
            env::var("BITCOIN_RPC_PASS").ok().or_else(|| config_string("rpc_pass")),
        ) {
            if !url.is_empty() && !user.is_empty() && !pass.is_empty() {
                wallet_cfg = wallet_cfg.with_rpc(&url, &user, &pass);
            }
        }

        node_config = node_config.with_wallet(wallet_cfg);
    }

    #[cfg(feature = "nostr")]
    {
        let relays: Vec<String> = env::var("BEENODE_RELAYS")
            .ok()
            .map(|s| s.split(',').map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect())
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|cfg| cfg.get("relays"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
            })
            .unwrap_or_default();

        if !relays.is_empty() {
            node_config = node_config.with_nostr(NostrConfig {
                relays,
                beebase_url: None,
                auto_connect: false,
            });
        }
    }

    Node::from_config(node_config).map_err(|e| format!("Failed to create node: {}", e))
}

fn cmd_init(opts: &ParsedArgs) -> Result<Value, String> {
    let app = opts.app.as_ref().ok_or("--app <name> is required")?;
    let mnemonic = opts.mnemonic.as_ref().ok_or("--mnemonic <words> is required")?;
    let auth_mode = parse_auth_mode(opts.auth_mode.as_deref())?;

    let pin = if auth_mode == AuthMode::Pin {
        let pin = prompt_pin()?;
        let mut auth = PinAuth::load(app).map_err(|e| format!("Auth load failed: {}", e))?;
        auth.set_pin(&pin, mnemonic)
            .map_err(|e| format!("Auth init failed: {}", e))?;
        Some(pin)
    } else {
        None
    };

    // Build and test node config
    #[allow(unused_mut)]
    let mut node_config = NodeConfig::new(app).with_auth_mode(auth_mode);

    if auth_mode == AuthMode::None {
        node_config = node_config.with_mnemonic(mnemonic);
    }

    #[cfg(feature = "wallet")]
    {
        let network = opts.network.as_deref().unwrap_or("signet");
        let net = match network {
            "bitcoin" | "mainnet" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Signet,
        };

        let mut wallet_cfg = WalletConfig {
            network: net,
            electrum_url: opts.electrum_url.clone(),
            data_dir: opts.data_dir.as_ref().map(std::path::PathBuf::from),
            #[cfg(feature = "bitcoind-rpc")]
            rpc: None,
        };

        // Use RPC if configured
        #[cfg(feature = "bitcoind-rpc")]
        if let (Some(ref url), Some(ref user), Some(ref pass)) = (&opts.rpc_url, &opts.rpc_user, &opts.rpc_pass) {
            wallet_cfg = wallet_cfg.with_rpc(url, user, pass);
        }

        node_config = node_config.with_wallet(wallet_cfg);
    }

    #[cfg(feature = "nostr")]
    if !opts.relays.is_empty() {
        node_config = node_config.with_nostr(NostrConfig {
            relays: opts.relays.clone(),
            beebase_url: None,
            auto_connect: false,
        });
    }

    // Test that node can be created and unlocked
    let node = Node::from_config(node_config).map_err(|e| format!("Init failed: {}", e))?;
    if let Some(ref pin) = pin {
        let unlocked = node.unlock(pin).map_err(|e| format!("Unlock failed: {}", e))?;
        if !unlocked {
            return Err("Invalid PIN".into());
        }
    }

    // Extract info
    let mobi = node.mobi().map(|m| m.display_formatted());
    let pubkey = node.pubkey_hex().map(|p| p[..16].to_string() + "...");

    node.close().ok();

    // Save config
    save_config(app, opts, auth_mode, Some(mnemonic))?;

    Ok(json!({
        "status": "initialized",
        "app": app,
        "config": config_path(app),
        "network": opts.network.as_deref().unwrap_or("signet"),
        "mobi": mobi,
        "pubkey": pubkey,
    }))
}

fn cmd_get(opts: &ParsedArgs) -> Result<Value, String> {
    let path = opts.path.as_ref().ok_or("Path required: beenode get <path>")?;
    let node = load_node_from_env()?;
    unlock_if_needed(&node, path, opts.pin.as_deref())?;

    let result = node.get(path).map_err(|e| format!("Get failed: {}", e))?;
    node.close().ok();

    match result {
        Some(scroll) => {
            if opts.scroll {
                Ok(json!({
                    "key": scroll.key,
                    "type": scroll.type_,
                    "metadata": {
                        "version": scroll.metadata.version,
                        "created_at": scroll.metadata.created_at,
                        "updated_at": scroll.metadata.updated_at,
                        "produced_by": scroll.metadata.produced_by,
                    },
                    "data": scroll.data,
                }))
            } else {
                Ok(json!({"data": scroll.data}))
            }
        }
        None => Err(format!("Not found: {}", path)),
    }
}

fn cmd_put(opts: &ParsedArgs) -> Result<Value, String> {
    let path = opts.path.as_ref().ok_or("Path required: beenode put <path> <json>")?;
    let data_str = opts.data.as_ref().ok_or("Data required: beenode put <path> <json>")?;

    let data: Value = serde_json::from_str(data_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let node = load_node_from_env()?;
    unlock_if_needed(&node, path, opts.pin.as_deref())?;
    let scroll = node.put(path, data).map_err(|e| format!("Put failed: {}", e))?;
    node.close().ok();

    if opts.scroll {
        Ok(json!({
            "key": scroll.key,
            "type": scroll.type_,
            "metadata": {
                "version": scroll.metadata.version,
            },
            "data": scroll.data,
        }))
    } else {
        Ok(json!({
            "status": "ok",
            "key": scroll.key,
            "version": scroll.metadata.version,
        }))
    }
}

fn cmd_list(opts: &ParsedArgs) -> Result<Value, String> {
    let prefix = opts.path.as_deref().unwrap_or("/");
    let node = load_node_from_env()?;
    unlock_if_needed(&node, prefix, opts.pin.as_deref())?;

    let paths = node.all(prefix).map_err(|e| format!("List failed: {}", e))?;
    node.close().ok();

    Ok(json!({
        "prefix": prefix,
        "paths": paths,
        "count": paths.len(),
    }))
}

fn cmd_repl(opts: &ParsedArgs) -> Result<Value, String> {
    println!("Beenode REPL - type 'help' or 'quit'\n");

    let node = load_node_from_env()?;
    if let Some(pin) = opts.pin.as_deref() {
        let _ = node.unlock(pin).map_err(|e| format!("Unlock failed: {}", e))?;
    }

    loop {
        print!("beenode> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.splitn(3, ' ').collect();

        match parts.get(0).copied() {
            Some("quit") | Some("exit") | Some("q") => break,
            Some("help") | Some("?") => {
                println!("Commands:");
                println!("  get <path>        - Read scroll");
                println!("  put <path> <json> - Write scroll");
                println!("  list [prefix]     - List paths");
                println!("  quit              - Exit");
            }
            Some("get") => {
                if let Some(path) = parts.get(1) {
                    match node.get(path) {
                        Ok(Some(s)) => println!("{}", serde_json::to_string_pretty(&s.data).unwrap()),
                        Ok(None) => println!("Not found: {}", path),
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("Usage: get <path>");
                }
            }
            Some("put") => {
                if parts.len() < 3 {
                    println!("Usage: put <path> <json>");
                    continue;
                }
                let path = parts[1];
                let json_str = parts[2];
                match serde_json::from_str::<Value>(json_str) {
                    Ok(data) => match node.put(path, data) {
                        Ok(s) => println!("OK (v{})", s.metadata.version),
                        Err(e) => println!("Error: {}", e),
                    },
                    Err(e) => println!("Invalid JSON: {}", e),
                }
            }
            Some("list") | Some("ls") => {
                let prefix = parts.get(1).copied().unwrap_or("/");
                match node.all(prefix) {
                    Ok(paths) => {
                        for p in &paths {
                            println!("{}", p);
                        }
                        println!("({} paths)", paths.len());
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            Some(cmd) => println!("Unknown: {}. Type 'help'.", cmd),
            None => {}
        }
    }

    node.close().ok();
    println!("Goodbye!");
    Ok(json!({"status": "exited"}))
}

fn cmd_serve(opts: &ParsedArgs) -> Result<Value, String> {
    use beenode::server::create_router_with_node;
    use beenode::clock::start_clock;
    use beenode::install_signal_handlers;
    use std::sync::Arc;

    let port = opts.port.unwrap_or(8080);
    let app_name = opts.app.clone().unwrap_or_else(|| "beenode".to_string());

    let node = load_node_from_env()?;
    if let Some(pin) = opts.pin.as_deref() {
        let _ = node.unlock(pin).map_err(|e| format!("Unlock failed: {}", e))?;
    }
    let node = Arc::new(node);

    // Create store for clock service
    let store = Arc::new(
        beenode::Store::open(&app_name, b"")
            .map_err(|e| format!("Failed to open store: {}", e))?
    );

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    rt.block_on(async {
        // Install signal handlers for graceful shutdown
        let shutdown = install_signal_handlers();

        // Start clock service (Layer 0 - boots first)
        let clock_handle = start_clock(store.clone(), shutdown.subscribe())
            .map_err(|e| format!("Failed to start clock: {}", e))?;
        info!("Clock service started (Layer 0)");

        let router = create_router_with_node(node, &app_name);
        let addr = format!("0.0.0.0:{}", port);

        info!("Beenode server listening on http://{}", addr);
        info!("Endpoints:");
        info!("  GET  /health              - Health check");
        info!("  GET  /scrolls?prefix=/    - List paths");
        info!("  GET  /sys/clock/tick      - Current clock tick");
        debug!("  GET  /scroll/*path        - Read scroll");
        debug!("  POST /scroll/*path        - Write scroll");

        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| format!("Failed to bind: {}", e))?;

        // Run server with graceful shutdown
        let mut shutdown_rx = shutdown.subscribe();
        tokio::select! {
            result = axum::serve(listener, router) => {
                result.map_err(|e| format!("Server error: {}", e))?;
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping server...");
            }
        }

        // Wait for clock to finish
        let _ = clock_handle.await;
        info!("Clock service stopped");

        Ok::<(), String>(())
    }).map_err(|e| format!("Server failed: {}", e))?;

    Ok(json!({"status": "stopped"}))
}

fn unlock_if_needed(node: &Node, path: &str, pin: Option<&str>) -> Result<(), String> {
    if node.is_locked() && !path.starts_with("/system/auth") {
        let pin = pin.ok_or("Node is locked. Provide --pin or call /system/auth/unlock.")?;
        let success = node.unlock(pin).map_err(|e| format!("Unlock failed: {}", e))?;
        if !success {
            return Err("Invalid PIN".into());
        }
    }
    Ok(())
}

fn prompt_pin() -> Result<String, String> {
    print!("Enter PIN: ");
    io::stdout().flush().ok();
    let mut pin = String::new();
    io::stdin().read_line(&mut pin).map_err(|e| format!("PIN read failed: {}", e))?;
    let pin = pin.trim().to_string();
    if pin.is_empty() {
        return Err("PIN cannot be empty".into());
    }
    Ok(pin)
}
