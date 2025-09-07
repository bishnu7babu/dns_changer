use anyhow::{anyhow, Result};
use dialoguer::{Select, Input};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct DnsProvider {
    name: String,
    primary_dns: String,
    secondary_dns: String,
    description: String,
}

struct DnsChanger {
    providers: Vec<DnsProvider>,
    current_connection: String,
}

impl DnsChanger {
    fn new() -> Result<Self> {
        let providers = vec![
            DnsProvider {
                name: "Cloudflare".to_string(),
                primary_dns: "1.1.1.1".to_string(),
                secondary_dns: "1.0.0.1".to_string(),
                description: "Fast and privacy-focused DNS".to_string(),
            },
            DnsProvider {
                name: "Google".to_string(),
                primary_dns: "8.8.8.8".to_string(),
                secondary_dns: "8.8.4.4".to_string(),
                description: "Reliable Google DNS".to_string(),
            },
            DnsProvider {
                name: "Quad9".to_string(),
                primary_dns: "9.9.9.9".to_string(),
                secondary_dns: "149.112.112.112".to_string(),
                description: "Security-focused DNS".to_string(),
            },
            DnsProvider {
                name: "OpenDNS".to_string(),
                primary_dns: "208.67.222.222".to_string(),
                secondary_dns: "208.67.220.220".to_string(),
                description: "Family-safe DNS".to_string(),
            },
        ];

        let current_connection = DnsChanger::get_active_connection()?;

        Ok(Self {
            providers,
            current_connection,
        })
    }

    fn get_active_connection() -> Result<String> {
        let output = Command::new("nmcli")
            .arg("-t")
            .arg("-f")
            .arg("NAME,DEVICE")
            .arg("connection")
            .arg("show")
            .arg("--active")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get active connections"));
        }

        let output_str = String::from_utf8(output.stdout)?;
        
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && !parts[1].is_empty() {
                return Ok(parts[0].to_string());
            }
        }

        Err(anyhow!("No active connection found"))
    }

    fn show_menu(&self) -> Result<()> {
        println!("========================================");
        println!("        Rust DNS Changer Tool");
        println!("========================================");
        println!("Current Connection: {}", self.current_connection);
        println!();

        let options = vec![
            "Select DNS Provider",
            "Custom DNS",
            "Automatic DNS (Router)",
            "Show Current DNS",
            "Exit",
        ];

        let selection = Select::new()
            .with_prompt("Choose an option")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.select_provider(),
            1 => self.set_custom_dns(),
            2 => self.set_automatic_dns(),
            3 => self.show_current_dns(),
            4 => {
                println!("Goodbye!");
                std::process::exit(1);
                // Ok(())
            }
            _ => Ok(()),
        }
    }

    fn select_provider(&self) -> Result<()> {
        let provider_names: Vec<String> = self.providers
            .iter()
            .map(|p| format!("{} - {}", p.name, p.description))
            .collect();

        let selection = Select::new()
            .with_prompt("Select DNS Provider")
            .items(&provider_names)
            .default(0)
            .interact()?;

        let provider = &self.providers[selection];
        self.set_dns(&provider.primary_dns, &provider.secondary_dns)?;
        
        println!("✅ DNS set to {} ({}, {})", 
            provider.name, provider.primary_dns, provider.secondary_dns);
        
        Ok(())
    }

    fn set_custom_dns(&self) -> Result<()> {
        let primary: String = Input::new()
            .with_prompt("Enter primary DNS")
            .interact_text()?;

        let secondary: String = Input::new()
            .with_prompt("Enter secondary DNS")
            .interact_text()?;

        self.set_dns(&primary, &secondary)?;
        println!("✅ DNS set to custom: {}, {}", primary, secondary);
        Ok(())
    }

    fn set_automatic_dns(&self) -> Result<()> {
        self.execute_command(&[
            "connection", "mod", &self.current_connection,
            "ipv4.dns", "",
            "ipv4.ignore-auto-dns", "no",
            "ipv6.ignore-auto-dns", "no"
        ])?;

        self.restart_connection()?;
        println!("✅ Switched to automatic DNS (Router)");
        Ok(())
    }

    fn set_dns(&self, primary: &str, secondary: &str) -> Result<()> {
        let dns = format!("{} {}", primary, secondary);
        
        self.execute_command(&[
            "connection", "mod", &self.current_connection,
            "ipv4.dns", &dns,
            "ipv4.ignore-auto-dns", "yes"
        ])?;

        self.restart_connection()?;
        Ok(())
    }

    fn execute_command(&self, args: &[&str]) -> Result<()> {
        let output = Command::new("sudo")
            .arg("nmcli")
            .args(args)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    fn restart_connection(&self) -> Result<()> {
        // Bring connection down
        let _ = Command::new("sudo")
            .arg("nmcli")
            .arg("connection")
            .arg("down")
            .arg(&self.current_connection)
            .output();

        // Bring connection up
        let output = Command::new("sudo")
            .arg("nmcli")
            .arg("connection")
            .arg("up")
            .arg(&self.current_connection)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to restart connection: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    fn show_current_dns(&self) -> Result<()> {
        let output = Command::new("nmcli")
            .arg("connection")
            .arg("show")
            .arg(&self.current_connection)
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8(output.stdout)?;
            for line in output_str.lines() {
                if line.contains("ipv4.dns") || line.contains("ipv4.ignore-auto-dns") {
                    println!("{}", line);
                }
            }
        }

        // Show system DNS info
        println!("\nSystem DNS configuration:");
        let _ = Command::new("resolvectl")
            .arg("status")
            .status();

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let dns_changer = DnsChanger::new()?;
    loop {
        if let Err(e) = dns_changer.show_menu() {
            eprintln!("Error: {}", e);
        }
        
        println!();
    }
}