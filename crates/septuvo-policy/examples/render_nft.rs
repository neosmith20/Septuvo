//! Lab tool (N1-03): print the nftables kill switch for a policy file.
//!
//! Usage: `render_nft <policy-file> <tunnel-interface> [endpoint ...]`
//! where each endpoint is `203.0.113.7:51820` or `[2001:db8::7]:51820`.
//! `render_nft --remove` prints the removal script.
//!
//! It only prints text; applying it is the lab harness's job.

use septuvo_policy::nft::{DnsRules, KillSwitch, removal_script, render};
use septuvo_policy::parse::MAX_INPUT_BYTES;
use septuvo_policy::parse_policy;
use std::io::Read;
use std::net::SocketAddr;
use std::process::ExitCode;

fn run(args: &[String]) -> Result<String, String> {
    if args == ["--remove"] {
        return Ok(removal_script());
    }
    let [path, interface, endpoints @ ..] = args else {
        return Err("usage: render_nft <policy-file> <tunnel-interface> [endpoint ...]".into());
    };
    let mut text = String::new();
    std::fs::File::open(path)
        .map_err(|e| format!("{path}: {e}"))?
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("{path}: {e}"))?;
    let policy = parse_policy(&text).map_err(|e| format!("{path}: {e}"))?;
    let endpoints = endpoints
        .iter()
        .map(|e| {
            e.parse::<SocketAddr>()
                .map(|a| (a.ip(), a.port()))
                .map_err(|_| format!("invalid endpoint {e:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ks = KillSwitch {
        tunnel_interface: interface.clone(),
        endpoints,
        dns: DnsRules::TunnelOnly,
    };
    render(&policy, &ks).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(script) => {
            print!("{script}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("render_nft: {e}");
            ExitCode::from(2)
        }
    }
}
