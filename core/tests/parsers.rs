//! The parsers, fed with what each system's tools print (serial numbers and MAC addresses made up).

use inventario_core::collect::{self, linux, macos, windows};
use inventario_core::net::{normalize_url, Trust};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn windows_lenovo_laptop() {
    let sample = json!({
        "hostname": "MAD-LT-014",
        "cs": {"Manufacturer": "LENOVO", "Model": "21HD003XSP", "PartOfDomain": true, "PCSystemType": 2},
        "product": {"Name": "21HD003XSP", "Version": "ThinkPad T14 Gen 4", "Vendor": "LENOVO"},
        "serial": "PF4ABCDE",
        "chassis": [10],
        "cpu": {"Name": "13th Gen Intel(R) Core(TM) i7-1355U", "NumberOfCores": 10, "MaxClockSpeed": 1700},
        "memory": [
            {"Capacity": 17179869184u64, "Speed": 5200, "ConfiguredClockSpeed": 5200, "SMBIOSMemoryType": 34, "FormFactor": 12},
            {"Capacity": 17179869184u64, "Speed": 5200, "ConfiguredClockSpeed": 5200, "SMBIOSMemoryType": 34, "FormFactor": 12}
        ],
        "gpus": [{"Name": "Intel(R) Iris(R) Xe Graphics", "AdapterRAM": 1073741824}, {"Name": "Microsoft Basic Display Adapter", "AdapterRAM": 0}],
        "disks": [
            {"Size": 512110190592u64, "MediaType": "SSD", "BusType": "NVMe"},
            {"Size": 64023257088u64, "MediaType": "Unspecified", "BusType": "USB"}
        ],
        "nics": [
            {"Description": "Intel(R) Wi-Fi 6E AX211 160MHz", "MacAddress": "A4-C3-F0-12-34-56", "Media": "Native 802.11"},
            {"Description": "Intel(R) Ethernet Connection (16) I219-V", "MacAddress": "8C-8C-AA-00-11-22", "Media": "802.3"},
            {"Description": "Bluetooth Device (Personal Area Network)", "MacAddress": "A4-C3-F0-12-34-5A", "Media": "BlueTooth"}
        ],
        "os": {"Caption": "Microsoft Windows 11 Pro", "DisplayVersion": "23H2", "ReleaseId": "2009", "Build": "22631"},
        "bitlocker": 1,
        "battery": {"Full": 47520, "Design": 52500},
        "screen": {"Width": 31, "Height": 17}
    });
    let mut f = windows::parse(&sample);
    collect::tidy(&mut f);
    assert_eq!(f.serial, "PF4ABCDE");
    assert_eq!(f.manufacturer, "Lenovo");
    assert_eq!(f.model, "ThinkPad T14 Gen 4");
    assert_eq!(f.chassis, "laptop");
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!(cpu.name, "Intel Core i7-1355U");
    assert_eq!(cpu.cores, Some(10));
    assert_eq!(cpu.max_ghz, Some(1.7));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!(ram.gb, Some(32.0));
    assert_eq!(ram.kind, "DDR5");
    assert_eq!(ram.mts, Some(5200.0));
    assert_eq!((ram.replaceable.as_str(), ram.modules, ram.module_gb, ram.form_factor.as_str()), ("yes", Some(2), Some(16.0), "SO-DIMM"));
    assert_eq!(f.disks.len(), 1, "the USB stick is left out");
    assert_eq!((f.disks[0].gb, f.disks[0].kind.as_str(), f.disks[0].connector.as_str()), (512.0, "SSD", "NVMe"));
    assert_eq!(f.gpus.len(), 1, "the basic display adapter is left out");
    assert_eq!(f.gpus[0].kind, "integrated");
    assert_eq!(f.nics.len(), 2, "Bluetooth is left out");
    assert_eq!(f.nics[0].mac, "A4:C3:F0:12:34:56");
    assert_eq!(f.nics[0].kind, "wifi");
    assert_eq!(f.nics[1].kind, "eth");
    let os = f.os.as_ref().unwrap();
    assert_eq!((os.family.as_str(), os.version.as_str(), os.edition.as_str()), ("win", "11 23H2", "Pro"));
    assert_eq!(f.encryption, "yes");
    assert_eq!(f.domain_joined, "yes");
    assert_eq!(f.battery.as_ref().unwrap().health_pct, 91.0);
    assert_eq!(f.screen_inches, Some(14.0));
    assert!(f.warnings.is_empty(), "{:?}", f.warnings);
}

#[test]
fn windows_desktop_without_details() {
    // a self-built desktop: placeholder serial and model, single module, no battery, BitLocker off, Windows 10
    let sample = json!({
        "hostname": "BCN-PC-03",
        "cs": {"Manufacturer": "System manufacturer", "Model": "System Product Name", "PartOfDomain": false, "PCSystemType": 1},
        "product": {"Name": "System Product Name", "Version": "System Version"},
        "serial": "System Serial Number",
        "chassis": 3,
        "cpu": {"Name": "AMD Ryzen 7 5800X 8-Core Processor", "NumberOfCores": 8, "MaxClockSpeed": 3801},
        "memory": {"Capacity": 17179869184u64, "Speed": 3200, "ConfiguredClockSpeed": 0, "SMBIOSMemoryType": 26, "FormFactor": 8},
        "gpus": {"Name": "NVIDIA GeForce RTX 3060", "AdapterRAM": 4293918720u64},
        "disks": [{"Size": 2000398934016u64, "MediaType": "HDD", "BusType": "SATA"}, {"Size": 1000204886016u64, "MediaType": "4", "BusType": "17"}],
        "nics": {"Description": "Realtek PCIe GbE Family Controller", "MacAddress": "00-D8-61-AA-BB-CC", "Media": "802.3"},
        "os": {"Caption": "Microsoft Windows 10 Home", "DisplayVersion": null, "ReleaseId": "2009"},
        "bitlocker": 2,
        "battery": {"Full": null, "Design": null},
        "screen": null
    });
    let mut f = windows::parse(&sample);
    collect::tidy(&mut f);
    assert_eq!(f.serial, "");
    assert_eq!(f.warnings, vec!["serial_missing"]);
    assert_eq!(f.model, "");
    assert_eq!(f.chassis, "desktop");
    assert_eq!(f.cpu.as_ref().unwrap().name, "AMD Ryzen 7 5800X");
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.form_factor.as_str()), (Some(16.0), "DDR4", Some(3200.0), "DIMM"));
    assert_eq!(f.gpus[0].kind, "dedicated");
    assert_eq!(f.disks.iter().map(|d| (d.kind.as_str(), d.connector.as_str())).collect::<Vec<_>>(), vec![("HDD", "SATA"), ("SSD", "NVMe")]);
    assert_eq!(f.os.as_ref().unwrap().version, "10 2009");
    assert_eq!(f.os.as_ref().unwrap().edition, "Home");
    assert_eq!(f.encryption, "no");
    assert_eq!(f.domain_joined, "no");
    assert!(f.battery.is_none());
}

#[test]
fn macos_intel_macbook_pro() {
    let profiler = json!({
        "SPHardwareDataType": [{
            "machine_name": "MacBook Pro", "machine_model": "MacBookPro15,1", "serial_number": "C02XK3JHMD6R",
            "number_processors": 6, "current_processor_speed": "2,6 GHz", "physical_memory": "16 GB"
        }],
        "SPMemoryDataType": [{
            "is_memory_upgradeable": "No",
            "_items": [
                {"_name": "BANK 0/ChannelA-DIMM0", "dimm_size": "8 GB", "dimm_type": "DDR4", "dimm_speed": "2400 MHz"},
                {"_name": "BANK 2/ChannelB-DIMM0", "dimm_size": "8 GB", "dimm_type": "DDR4", "dimm_speed": "2400 MHz"}
            ]
        }],
        "SPNVMeDataType": [{"_items": [{"_name": "APPLE SSD AP0512M", "size_in_bytes": 500277790720u64, "detachable_drive": "no"}]}],
        "SPSerialATADataType": [],
        "SPDisplaysDataType": [
            {"sppci_model": "Intel UHD Graphics 630", "spdisplays_vendor": "Intel"},
            {"sppci_model": "Radeon Pro 560X", "spdisplays_vendor": "sppci_vendor_amd", "spdisplays_vram": "4 GB"}
        ],
        "SPPowerDataType": [{"sppower_battery_health_info": {"sppower_battery_health_maximum_capacity": "87%"}}]
    });
    let mut f = macos::parse(&macos::Inputs {
        profiler,
        cpu_brand: "Intel(R) Core(TM) i7-8850H CPU @ 2.60GHz\n".into(),
        product_version: "14.6.1\n".into(),
        hardware_ports: "\nHardware Port: Wi-Fi\nDevice: en0\nEthernet Address: 3c:22:fb:01:02:03\n\nHardware Port: Thunderbolt Bridge\nDevice: bridge0\nEthernet Address: N/A\n\nHardware Port: USB 10/100/1000 LAN\nDevice: en5\nEthernet Address: 00:e0:4c:68:00:01\n".into(),
        filevault: "FileVault is Off.\n".into(),
        directory: Some(String::new()),
        ioreg_battery: String::new(),
        hostname: "MBP-de-Ana".into(),
        screen_mm: Some((331.0, 207.0)), // what CoreGraphics reports for a 15.4" MacBook Pro
    });
    collect::tidy(&mut f);
    assert_eq!(f.serial, "C02XK3JHMD6R");
    assert_eq!(f.model, "MacBook Pro (MacBookPro15,1)");
    assert_eq!(f.chassis, "laptop");
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!((cpu.name.as_str(), cpu.cores, cpu.max_ghz), ("Intel Core i7-8850H", Some(6), Some(2.6)));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.replaceable.as_str()), (Some(16.0), "DDR4", Some(2400.0), "no"));
    assert_eq!((f.disks[0].gb, f.disks[0].connector.as_str()), (500.0, "NVMe"));
    assert_eq!(f.gpus.len(), 2);
    assert_eq!((f.gpus[1].kind.as_str(), f.gpus[1].vram_gb), ("dedicated", Some(4.0)));
    assert_eq!(f.battery.as_ref().unwrap().health_pct, 87.0);
    assert_eq!(f.screen_inches, Some(15.4));
    assert_eq!(f.os.as_ref().unwrap().version, "14 Sonoma");
    assert_eq!(f.encryption, "no");
    assert_eq!(f.domain_joined, "no");
    assert_eq!(f.nics.len(), 1, "only the built-in Wi-Fi; USB adapters and bridges are left out");
    assert_eq!(f.nics[0].mac, "3C:22:FB:01:02:03");
}

#[test]
fn macos_version_names() {
    assert_eq!(macos::version_name("10.15.7"), "10.15 Catalina");
    assert_eq!(macos::version_name("11.7.10"), "11 Big Sur");
    assert_eq!(macos::version_name("15.1"), "15 Sequoia");
    assert_eq!(macos::version_name("26.0"), "26 Tahoe");
    assert_eq!(macos::version_name("27.2"), "27");
    assert_eq!(macos::version_name(""), "");
}

fn ubuntu_inputs(is_root: bool) -> linux::Inputs {
    let mut dmi = HashMap::new();
    dmi.insert("sys_vendor".into(), "Dell Inc.".into());
    dmi.insert("product_name".into(), "Latitude 5440".into());
    dmi.insert("chassis_type".into(), "10".into());
    if is_root {
        dmi.insert("product_serial".into(), "8XYZ123".into());
    }
    linux::Inputs {
        dmi,
        lscpu: json!({"lscpu": [
            {"field": "Architecture:", "data": "x86_64"},
            {"field": "Vendor ID:", "data": "GenuineIntel", "children": [
                {"field": "Model name:", "data": "13th Gen Intel(R) Core(TM) i5-1345U", "children": [
                    {"field": "Core(s) per socket:", "data": "10"}, {"field": "Socket(s):", "data": "1"}, {"field": "CPU max MHz:", "data": "4700,0000"}
                ]}
            ]}
        ]}),
        meminfo: "MemTotal:       16076584 kB\nMemFree:  1000 kB\n".into(),
        dmidecode_memory: if is_root {
            "Handle 0x0040, DMI type 17\nMemory Device\n\tSize: 8 GB\n\tForm Factor: SODIMM\n\tType: DDR4\n\tSpeed: 3200 MT/s\n\tConfigured Memory Speed: 3200 MT/s\n\nHandle 0x0041, DMI type 17\nMemory Device\n\tSize: 8 GB\n\tForm Factor: SODIMM\n\tType: DDR4\n\tSpeed: 3200 MT/s\n".into()
        } else { String::new() },
        lsblk: json!({"blockdevices": [
            {"name": "nvme0n1", "size": 512110190592u64, "rota": false, "tran": "nvme", "type": "disk", "rm": false, "children": [
                {"name": "nvme0n1p1", "size": 1073741824, "rota": false, "tran": null, "type": "part", "rm": false},
                {"name": "nvme0n1p3", "size": 510000000000u64, "rota": false, "tran": null, "type": "part", "rm": false, "children": [
                    {"name": "dm_crypt-0", "size": 509000000000u64, "rota": false, "tran": null, "type": "crypt", "rm": false}
                ]}
            ]},
            {"name": "sda", "size": 32010928128u64, "rota": true, "tran": "usb", "type": "disk", "rm": true},
            {"name": "loop0", "size": 4096, "rota": false, "tran": null, "type": "loop", "rm": false}
        ]}),
        lspci: "00:02.0 \"VGA compatible controller\" \"Intel Corporation\" \"Raptor Lake-P [Iris Xe Graphics]\" -r04 \"Dell\" \"Device 0c0d\"\n00:14.3 \"Network controller\" \"Intel Corporation\" \"Raptor Lake PCH CNVi WiFi\" \"Intel Corporation\" \"Wi-Fi 6E AX211\"\n".into(),
        nets: vec![
            linux::NetDevice { name: "lo".into(), mac: "00:00:00:00:00:00".into(), wireless: false, physical: false },
            linux::NetDevice { name: "wlp0s20f3".into(), mac: "a4:c3:f0:aa:bb:cc".into(), wireless: true, physical: true },
            linux::NetDevice { name: "enp0s31f6".into(), mac: "8c:8c:aa:dd:ee:ff".into(), wireless: false, physical: true },
            linux::NetDevice { name: "docker0".into(), mac: "02:42:ac:11:00:01".into(), wireless: false, physical: false },
        ],
        os_release: "PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nNAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nVERSION=\"24.04.1 LTS (Noble Numbat)\"\nID=ubuntu\n".into(),
        realm: None,
        battery: Some((48000000.0, 54000000.0)),
        edid: Some({ let mut e = vec![0u8; 128]; e[21] = 31; e[22] = 17; e }),
        hostname: "dell-ana".into(),
        is_root,
        ..Default::default()
    }
}

#[test]
fn linux_dell_as_root() {
    let mut f = linux::parse(&ubuntu_inputs(true));
    collect::tidy(&mut f);
    assert_eq!(f.serial, "8XYZ123");
    assert_eq!((f.manufacturer.as_str(), f.model.as_str(), f.chassis.as_str()), ("Dell", "Latitude 5440", "laptop"));
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!((cpu.name.as_str(), cpu.cores, cpu.max_ghz), ("Intel Core i5-1345U", Some(10), Some(4.7)));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.modules, ram.module_gb, ram.form_factor.as_str()),
               (Some(16.0), "DDR4", Some(3200.0), Some(2), Some(8.0), "SO-DIMM"));
    assert_eq!(f.disks.len(), 1, "the USB stick and loop devices are left out");
    assert_eq!((f.disks[0].gb, f.disks[0].connector.as_str()), (512.0, "NVMe"));
    assert_eq!(f.gpus.len(), 1, "network controllers are not graphics");
    assert_eq!((f.gpus[0].name.as_str(), f.gpus[0].kind.as_str()), ("Intel Iris Xe Graphics", "integrated"));
    assert_eq!(f.nics.iter().map(|n| (n.kind.as_str(), n.mac.as_str())).collect::<Vec<_>>(),
               vec![("wifi", "A4:C3:F0:AA:BB:CC"), ("eth", "8C:8C:AA:DD:EE:FF")]);
    let os = f.os.as_ref().unwrap();
    assert_eq!((os.family.as_str(), os.distro.as_str(), os.version.as_str()), ("linux", "Ubuntu", "24.04 LTS"));
    assert_eq!(f.encryption, "yes");
    assert_eq!(f.domain_joined, "");
    assert_eq!(f.battery.as_ref().unwrap().health_pct, 89.0);
    assert_eq!(f.screen_inches, Some(14.0));
    assert!(f.warnings.is_empty(), "{:?}", f.warnings);
}

#[test]
fn linux_without_root_asks_for_serial() {
    let mut f = linux::parse(&ubuntu_inputs(false));
    collect::tidy(&mut f);
    assert_eq!(f.serial, "");
    assert_eq!(f.warnings, vec!["serial_needs_admin"], "one warning, not also serial_missing");
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.modules), (Some(16.0), "", None));
}

#[test]
fn linux_distributions() {
    assert_eq!(linux::distro_and_version("NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\nVERSION=\"12 (bookworm)\"\n"), ("Debian".into(), "12".into()));
    assert_eq!(linux::distro_and_version("NAME=\"Fedora Linux\"\nVERSION_ID=40\nVERSION=\"40 (Workstation Edition)\"\n"), ("Fedora".into(), "40".into()));
    assert_eq!(linux::distro_and_version("NAME=\"Arch Linux\"\nBUILD_ID=rolling\n"), ("Arch Linux".into(), "".into()));
}

#[test]
fn raspberry_pi() {
    let f = linux::parse(&linux::Inputs {
        devicetree_serial: "10000000abcdef01\u{0}".trim_end_matches('\u{0}').into(),
        cpuinfo: "processor\t: 0\nModel\t\t: Raspberry Pi 4 Model B Rev 1.4\n".into(),
        lscpu: json!({"lscpu": [{"field": "Vendor ID:", "data": "ARM"}, {"field": "Model name:", "data": "Cortex-A72"},
                                {"field": "Core(s) per cluster:", "data": "4"}]}),
        os_release: "NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\n".into(),
        ..Default::default()
    });
    assert_eq!(f.serial, "10000000abcdef01");
    assert_eq!(f.model, "Raspberry Pi 4 Model B Rev 1.4");
    assert_eq!(f.cpu.as_ref().unwrap().name, "Cortex-A72");
    assert_eq!(f.cpu.as_ref().unwrap().cores, Some(4));
    assert_eq!(f.gpus[0].name, "Broadcom VideoCore");
}

#[test]
fn addresses() {
    assert_eq!(normalize_url("inventario.empresa.es").unwrap(), "https://inventario.empresa.es");
    assert_eq!(normalize_url(" https://192.168.1.20:8443/ ").unwrap(), "https://192.168.1.20:8443");
    assert_eq!(normalize_url("HTTPS://Inventario.local/#panel").unwrap(), "https://Inventario.local");
    assert_eq!(normalize_url("http://localhost:8000").unwrap(), "http://localhost:8000");
    assert_eq!(normalize_url("http://[::1]:8000").unwrap(), "http://[::1]:8000");
    assert!(normalize_url("http://inventario.empresa.es").is_err(), "no passwords over plain http");
    assert!(normalize_url("ftp://x").is_err());
    assert!(normalize_url("").is_err());
    assert!(normalize_url("https://").is_err());
}

#[test]
fn fingerprints_compare_however_written() {
    let trust = Trust { fingerprint: "53:BE:AE:DC".into(), ca: String::new() };
    assert!(trust.matches("53beaedc"));
    assert!(trust.matches("53 be ae dc"));
    assert!(!trust.matches("53beaedd"));
    assert!(!Trust::default().matches(""));
}
