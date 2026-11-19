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
        "wlan": "Interface name: Wi-Fi\n    Driver : Intel(R) Wi-Fi 6E AX211 160MHz\n    Radio types supported : 802.11b 802.11g 802.11n 802.11a 802.11ac 802.11ax\n    Wireless Display Supported: Yes",
        "lmp": 12,
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
    assert_eq!((f.nics[0].wifi_gen.as_str(), f.nics[0].bt_ver.as_str()), ("6E", "5.3"), "6E from the card's name");
    assert_eq!(f.nics[1].kind, "eth");
    assert_eq!(f.nics[1].wifi_gen, "", "only Wi-Fi has a generation");
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
        "SPPowerDataType": [{"sppower_battery_health_info": {"sppower_battery_health_maximum_capacity": "87%"}}],
        "SPAirPortDataType": [{"spairport_airport_interfaces": [
            {"_name": "en0", "spairport_supported_phymodes": "802.11 a/b/g/n/ac", "spairport_supported_channels": ["1 (2GHz)", "36 (5GHz)"],
             "spairport_wireless_card_type": "spairport_wireless_card_type_wifi (0x14E4, 0x7BF)"},
            {"_name": "awdl0"}
        ]}]
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
        nvram_oclp: String::new(),
        nvram_opencore: String::new(),
        oclp_patches: String::new(),
        media: Default::default(),
    });
    collect::tidy(&mut f);
    assert_eq!(f.serial, "C02XK3JHMD6R");
    assert_eq!(f.model, "MacBook Pro (MacBookPro15,1)");
    assert_eq!(f.chassis, "laptop");
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!((cpu.name.as_str(), cpu.cores, cpu.max_ghz), ("Intel Core i7-8850H", Some(6), Some(2.6)));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.replaceable.as_str()), (Some(16.0), "DDR4", Some(2400.0), "no"));
    assert_eq!((f.disks[0].gb, f.disks[0].connector.as_str(), f.disks[0].replaceable.as_str()), (512.0, "NVMe", "no")); // a T2 Mac; "APPLE SSD AP0512M" is the 512 GB model
    assert_eq!(f.oclp.as_deref(), Some(""));
    assert_eq!(f.gpus.len(), 2);
    assert_eq!((f.gpus[1].kind.as_str(), f.gpus[1].vram_gb), ("dedicated", Some(4.0)));
    assert_eq!(f.battery.as_ref().unwrap().health_pct, 87.0);
    assert_eq!(f.screen_inches, Some(15.4));
    assert_eq!(f.os.as_ref().unwrap().version, "14 Sonoma");
    assert_eq!(f.encryption, "no");
    assert_eq!(f.domain_joined, "no");
    assert_eq!(f.nics.len(), 1, "only the built-in Wi-Fi; USB adapters and bridges are left out");
    assert_eq!(f.nics[0].mac, "3C:22:FB:01:02:03");
    assert_eq!((f.nics[0].model.as_str(), f.nics[0].wifi_gen.as_str()), ("Broadcom", "5"));
}

#[test]
fn macos_version_names() {
    assert_eq!(macos::version_name("10.15.7"), "10.15 Catalina");
    assert_eq!(macos::version_name("11.7.10"), "11 Big Sur");
    assert_eq!(macos::version_name("15.1"), "15 Sequoia");
    assert_eq!(macos::version_name("26.0"), "26 Tahoe");
    assert_eq!(macos::version_name("27.2"), "27");
    assert_eq!(macos::version_name(""), "");
    assert_eq!(macos::version_number("27.2.1\n"), "27.2.1");
    assert_eq!(macos::version_number("26.0"), "26.0");
    assert_eq!(macos::version_number(""), "");
    assert_eq!(macos::version_number("26.2 beta"), "");
    use inventario_core::facts::Os;
    let os = |version: &str, number: &str| Os { family: "mac".into(), version: version.into(), number: number.into(), ..Os::default() }.full_version();
    assert_eq!(os("26 Tahoe", "26.2.1"), "26.2.1 Tahoe");
    assert_eq!(os("10.15 Catalina", "10.15.7"), "10.15.7 Catalina");
    assert_eq!(os("27", "27.2.1"), "27.2.1");
    assert_eq!(os("26 Tahoe", ""), "26 Tahoe");
    assert_eq!(os("1 X", "10.2"), "1 X (10.2)", "10.2 is not release 1");
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
        lspci: "00:02.0 \"VGA compatible controller\" \"Intel Corporation\" \"Raptor Lake-P [Iris Xe Graphics]\" -r04 \"Dell\" \"Device 0c0d\"\n00:14.3 \"Network controller\" \"Intel Corporation\" \"Raptor Lake PCH CNVi WiFi\" -r01 \"Intel Corporation\" \"Wi-Fi 6E AX211 160MHz\"\n00:1f.6 \"Ethernet controller\" \"Intel Corporation\" \"Ethernet Connection (16) I219-LM\" -r01 \"Dell\" \"Device 0c0d\"\n".into(),
        nets: vec![
            linux::NetDevice { name: "lo".into(), mac: "00:00:00:00:00:00".into(), wireless: false, physical: false, pci_slot: String::new(), usb: false },
            linux::NetDevice { name: "wlp0s20f3".into(), mac: "a4:c3:f0:aa:bb:cc".into(), wireless: true, physical: true, pci_slot: "0000:00:14.3".into(), usb: false },
            linux::NetDevice { name: "enp0s31f6".into(), mac: "8c:8c:aa:dd:ee:ff".into(), wireless: false, physical: true, pci_slot: "0000:00:1f.6".into(), usb: false },
            linux::NetDevice { name: "docker0".into(), mac: "02:42:ac:11:00:01".into(), wireless: false, physical: false, pci_slot: String::new(), usb: false },
        ],
        os_release: "PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nNAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nVERSION=\"24.04.1 LTS (Noble Numbat)\"\nID=ubuntu\n".into(),
        realm: None,
        battery: Some((48000000.0, 54000000.0)),
        edid: Some({ let mut e = vec![0u8; 128]; e[21] = 31; e[22] = 17; e }),
        hostname: "dell-ana".into(),
        is_root,
        iw: "Wiphy phy0\n\tBand 1:\n\t\tHT Capabilities: 0x1ef\n\t\tHE Iftypes: managed\n\tBand 2:\n\t\tVHT Capabilities (0x039071f6):\n\tBand 4:\n\t\tHE Iftypes: managed\n".into(),
        hci: "hci0:\tType: Primary  Bus: USB\n\tHCI Version: 5.3 (0xc)  Revision: 0x0\n\tLMP Version: 5.3 (0xc)  Subversion: 0x0\n".into(),
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
    assert_eq!(f.nics.iter().map(|n| (n.kind.as_str(), n.mac.as_str(), n.model.as_str(), n.wifi_gen.as_str(), n.bt_ver.as_str())).collect::<Vec<_>>(),
               vec![("wifi", "A4:C3:F0:AA:BB:CC", "Intel Wi-Fi 6E AX211 160MHz", "6E", "5.3"),
                    ("eth", "8C:8C:AA:DD:EE:FF", "Intel Ethernet Connection (16) I219-LM", "", "")]);
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

#[test]
fn wifi_generations() {
    use inventario_core::collect::util::{bt_from_lmp, wifi_gen_from_name, wifi_gen_from_standards};
    assert_eq!(wifi_gen_from_standards("802.11 a/b/g/n/ac/ax", true), "6E");
    assert_eq!(wifi_gen_from_standards("802.11 a/b/g/n/ac/ax", false), "6");
    assert_eq!(wifi_gen_from_standards("Tipos de radio admitidos: 802.11b 802.11g 802.11n 802.11a 802.11ac", false), "5");
    assert_eq!(wifi_gen_from_standards("802.11be 802.11ax", true), "7");
    assert_eq!(wifi_gen_from_standards("Hosted network supported : No, maybe", false), "", "words outside 802.11 names are ignored");
    assert_eq!(wifi_gen_from_name("Killer(R) Wi-Fi 7 BE1750x 320MHz"), "7");
    assert_eq!(wifi_gen_from_name("Intel(R) Wi-Fi 6 AX201 160MHz"), "6");
    assert_eq!(wifi_gen_from_name("Realtek RTL8852BE WiFi 6 802.11ax PCIe Adapter"), "6");
    assert_eq!(wifi_gen_from_name("Realtek PCIe GbE Family Controller"), "");
    assert_eq!(macos::wifi_chip("spairport_wireless_card_type_wifi (0x14C3, 0x7932)"), "MediaTek MT7932");
    assert_eq!(macos::wifi_chip("Wi-Fi"), "");
    assert_eq!((bt_from_lmp(12), bt_from_lmp(99)), ("5.3".to_string(), String::new()));
}

#[test]
fn drives_that_can_be_replaced() {
    use inventario_core::collect::util::{mac_disk_replaceable as mac, pc_disk_replaceable as pc};
    assert_eq!(mac("iMac14,2", false), "yes"); // 2013 iMac: a SATA drive
    assert_eq!(mac("MacBookPro11,3", false), "yes"); // an SSD on a removable card
    assert_eq!(mac("MacBookPro13,1", false), "yes"); // 2016 13-inch without Touch Bar
    assert_eq!(mac("MacBookPro13,2", false), "no"); // with Touch Bar: soldered
    assert_eq!(mac("MacBookPro15,1", false), "no"); // T2
    assert_eq!(mac("MacBookAir7,2", false), "yes");
    assert_eq!(mac("MacBookAir8,1", false), "no");
    assert_eq!(mac("MacBook10,1", false), "no");
    assert_eq!(mac("iMac19,1", false), "yes");
    assert_eq!(mac("iMac20,1", false), "no");
    assert_eq!(mac("Macmini7,1", false), "yes");
    assert_eq!(mac("Macmini8,1", false), "no");
    assert_eq!(mac("Mac14,2", true), "no"); // Apple silicon
    assert_eq!(mac("Xserve3,1", false), "");
    assert_eq!(pc("SSD", "NVMe", "NVMe", "Samsung SSD 980 PRO 1TB"), "yes");
    assert_eq!(pc("SSD", "NVMe", "NVMe", "KIOXIA BG4 BGA 256GB"), "no");
    assert_eq!(pc("HDD", "SATA", "SATA", "ST1000LM035"), "yes");
    assert_eq!(pc("eMMC", "", "MMC", "DA4064"), "no");
    assert_eq!(pc("SSD", "", "UFS", "KLUDG4UHDB"), "no");
    assert_eq!(pc("SSD", "", "", ""), "");
}

#[test]
fn opencore_legacy_patcher() {
    let mac = |oclp: &str, opencore: &str, patches: &str| {
        macos::oclp(&macos::Inputs {
            profiler: serde_json::Value::Null, cpu_brand: String::new(), product_version: String::new(),
            hardware_ports: String::new(), filevault: String::new(), directory: None, ioreg_battery: String::new(),
            hostname: String::new(), screen_mm: None,
            nvram_oclp: oclp.into(), nvram_opencore: opencore.into(), oclp_patches: patches.into(), media: Default::default(),
        })
    };
    let guid = "4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102";
    assert_eq!(mac(&format!("{guid}:OCLP-Version\t2.1.0%00\n"), &format!("{guid}:opencore-version\tREL-103-2024-12-01\n"), ""), "2.1.0");
    assert_eq!(mac("", "", r#"{"OpenCore Legacy Patcher": "v1.5.0", "PatcherSupportPkg": "1.4.9"}"#), "1.5.0");
    assert_eq!(mac("", &format!("{guid}:opencore-version\tREL-103-2024-12-01\n"), ""), "yes");
    assert_eq!(mac("", "", ""), "");
}

#[test]
fn windows_battery_warnings() {
    let laptop = |battery: serde_json::Value| {
        let f = windows::parse(&serde_json::json!({ "serial": "PF4ABCDE", "chassis": [10], "battery": battery }));
        (f.battery.map(|b| b.health_pct), f.warnings)
    };
    // the figures from WMI, or from Windows' battery report (powercfg) when WMI had none
    assert_eq!(laptop(serde_json::json!({"Full": 45000.0, "Design": 50000.0, "Present": true, "Admin": false})), (Some(90.0), vec![]));
    // nothing anywhere: only suggest administrator rights when the app did not have them
    assert_eq!(laptop(serde_json::json!({"Full": null, "Design": null, "Present": true, "Admin": false})).1, vec!["battery_needs_admin"]);
    assert_eq!(laptop(serde_json::json!({"Full": null, "Design": null, "Present": true, "Admin": true})).1, vec!["battery_unknown"]);
}

#[test]
fn mac_memory_modules() {
    let mac = |identifier: &str, machine: &str| {
        let profiler = serde_json::json!({
            "SPHardwareDataType": [{"machine_model": identifier, "machine_name": machine, "physical_memory": "16 GB"}],
            "SPMemoryDataType": [{"is_memory_upgradeable": "Yes", "_items": [
                {"dimm_size": "8 GB", "dimm_type": "DDR3", "dimm_speed": "1600 MHz"},
                {"dimm_size": "8 GB", "dimm_type": "DDR3", "dimm_speed": "1600 MHz"}
            ]}]
        });
        let f = macos::parse(&macos::Inputs {
            profiler, cpu_brand: "Intel(R) Core(TM) i5-4670 CPU @ 3.40GHz".into(), product_version: String::new(),
            hardware_ports: String::new(), filevault: String::new(), directory: None, ioreg_battery: String::new(),
            hostname: String::new(), screen_mm: None, nvram_oclp: String::new(), nvram_opencore: String::new(), oclp_patches: String::new(),
            media: Default::default(),
        });
        f.ram.unwrap().form_factor
    };
    assert_eq!(mac("iMac14,2", "iMac"), "SO-DIMM");
    assert_eq!(mac("Macmini7,1", "Mac mini"), "SO-DIMM");
    assert_eq!(mac("MacBookPro9,2", "MacBook Pro"), "SO-DIMM");
    assert_eq!(mac("MacPro5,1", "Mac Pro"), "DIMM");
}

#[test]
fn drive_generation_and_form() {
    use inventario_core::collect::util::{mac_disk_form, max_ethernet_mbps, nvme_connector, pc_disk_form, sata_connector};
    assert_eq!((sata_connector("6 Gigabit"), sata_connector("3.0 Gb/s"), sata_connector("1.5 Gbps"), sata_connector("")), ("SATA III", "SATA II", "SATA I", "SATA"));
    assert_eq!((nvme_connector("8.0 GT/s PCIe"), nvme_connector("16.0 GT/s PCIe"), nvme_connector("Unknown")), ("NVMe PCIe 3.0".to_string(), "NVMe PCIe 4.0".to_string(), "NVMe".to_string()));
    assert_eq!(pc_disk_form("SSD", "NVMe PCIe 4.0", "yes", "laptop"), "M.2");
    assert_eq!(pc_disk_form("SSD", "NVMe", "no", "laptop"), ""); // soldered
    assert_eq!((pc_disk_form("HDD", "SATA III", "yes", "laptop"), pc_disk_form("HDD", "SATA III", "yes", "desktop")), ("2.5\"", "3.5\""));
    assert_eq!(pc_disk_form("SSD", "SATA III", "yes", "laptop"), ""); // 2.5", M.2 or mSATA: not said
    assert_eq!(mac_disk_form("iMac14,2", "SSD", "SATA III", "yes", Some(27.0)), "2.5\"");
    assert_eq!(mac_disk_form("iMac14,2", "HDD", "SATA III", "yes", Some(27.0)), "3.5\"");
    assert_eq!(mac_disk_form("iMac14,1", "HDD", "SATA III", "yes", Some(21.5)), "2.5\"");
    assert_eq!(mac_disk_form("iMac12,1", "HDD", "SATA II", "yes", Some(21.5)), "3.5\"");
    assert_eq!(mac_disk_form("MacBookPro11,3", "SSD", "NVMe", "yes", None), "Apple (propietario)");
    assert_eq!(mac_disk_form("Mac14,2", "SSD", "NVMe", "no", None), "");
    assert_eq!(max_ethernet_mbps("supported media:\n\tmedia autoselect\n\tmedia 10baseT/UTP <full-duplex>\n\tmedia 100baseTX <full-duplex>\n\tmedia 1000baseT <full-duplex>"), Some(1000));
    assert_eq!(max_ethernet_mbps("Supported link modes:   10baseT/Half 10baseT/Full\n 100baseT/Full 1000baseT/Full\n 2500baseT/Full"), Some(2500));
    assert_eq!(max_ethernet_mbps("media 10Gbase-T <full-duplex>"), Some(10000));
    assert_eq!(max_ethernet_mbps("Auto Negotiation;10 Mbps Half Duplex;100 Mbps Full Duplex;1.0 Gbps Full Duplex"), Some(1000));
    assert_eq!(max_ethernet_mbps("Auto Negotiation"), None);
}

#[test]
fn mac_sata_and_ethernet() {
    let profiler = serde_json::json!({
        "SPHardwareDataType": [{"machine_model": "iMac14,2", "machine_name": "iMac"}],
        "SPSerialATADataType": [{"_name": "Intel 8 Series Chipset", "spsata_negotiatedlinkspeed": "6 Gigabit",
            "_items": [{"_name": "Samsung SSD 860 EVO 1TB", "size_in_bytes": 1000204886016u64, "spsata_medium_type": "Solid State"}]}],
        "SPEthernetDataType": [{"_name": "ethernet", "spethernet_BSD_Device_Name": "en0", "spethernet_vendor-id": "0x14e4", "spethernet_device-id": "0x16b4"}]
    });
    let mut media = std::collections::HashMap::new();
    media.insert("en0".to_string(), "supported media:\n\tmedia 10baseT/UTP <full-duplex>\n\tmedia 1000baseT <full-duplex>".to_string());
    let f = macos::parse(&macos::Inputs {
        profiler, cpu_brand: "Intel(R) Core(TM) i5-4670 CPU @ 3.40GHz".into(), product_version: String::new(),
        hardware_ports: "Hardware Port: Ethernet\nDevice: en0\nEthernet Address: 10:dd:b1:f1:15:d0\n".into(), filevault: String::new(),
        directory: None, ioreg_battery: String::new(), hostname: String::new(), screen_mm: Some((597.0, 336.0)),
        nvram_oclp: String::new(), nvram_opencore: String::new(), oclp_patches: String::new(), media,
    });
    let disk = &f.disks[0];
    assert_eq!((disk.connector.as_str(), disk.form_factor.as_str(), disk.replaceable.as_str()), ("SATA III", "2.5\"", "yes"));
    assert_eq!((f.nics[0].model.as_str(), f.nics[0].speed_mbps), ("Broadcom BCM57765", Some(1000)));
}

#[test]
fn ethernet_speed_from_the_name() {
    use inventario_core::collect::util::ethernet_mbps_from_name as by_name;
    assert_eq!(by_name("Realtek USB GbE Family Controller"), Some(1000));
    assert_eq!(by_name("Intel(R) Ethernet Connection (16) I219-V"), None); // says nothing: left out
    assert_eq!(by_name("Realtek Gaming 2.5GbE Family Controller"), Some(2500));
    assert_eq!(by_name("Intel(R) Ethernet Controller I225-V 2.5G"), Some(2500));
    assert_eq!(by_name("Aquantia AQC107 10GbE"), Some(10000));
    assert_eq!(by_name("Realtek PCIe FE Family Controller Fast Ethernet"), Some(100));
    assert_eq!(by_name("ASIX AX88179 USB 3.0 to Gigabit Ethernet Adapter"), Some(1000));
}

#[test]
fn sizes_as_sold() {
    use inventario_core::collect::util::{disk_gb, disk_gb_sold, ram_gb_from_bytes};
    let gib = 1024.0 * 1024.0 * 1024.0;
    assert_eq!(disk_gb(1_024_209_543_168.0), 1000.0); // a "1 TB" drive that holds 1,024 GB
    assert_eq!(disk_gb(256_060_514_304.0), 256.0);
    assert_eq!(disk_gb(500_107_862_016.0), 500.0); // a 500 GB drive stays 500
    assert_eq!(disk_gb(2_048_408_248_320.0), 2000.0);
    assert_eq!(disk_gb(150_100_000_000.0), 150.0); // no size sold within 3%: the exact size
    assert_eq!(disk_gb_sold(500_277_792_768.0, true), 512.0); // Apple's "512 GB"
    assert_eq!(disk_gb_sold(251_000_193_024.0, true), 256.0);
    assert_eq!(disk_gb_sold(1_000_555_581_440.0, true), 1000.0);
    assert_eq!(ram_gb_from_bytes(15.6 * gib), 16.0);
    assert_eq!(ram_gb_from_bytes(7.7 * gib), 8.0);
    assert_eq!(ram_gb_from_bytes(2.9 * gib), 3.0);
    assert_eq!(ram_gb_from_bytes(11.8 * gib), 12.0);
    assert_eq!(ram_gb_from_bytes(17.9 * gib), 18.0);
}

#[test]
fn graphics_kind() {
    use windows::gpu_kind;
    assert_eq!(gpu_kind("intel arc 140v gpu (16gb)"), "integrated"); // Lunar Lake: built in
    assert_eq!(gpu_kind("intel arc graphics"), "integrated"); // Meteor Lake: built in
    assert_eq!(gpu_kind("intel arc a770 graphics"), "dedicated");
    assert_eq!(gpu_kind("intel arc b580 graphics"), "dedicated");
    assert_eq!(gpu_kind("intel arc pro a60m graphics"), "dedicated");
    assert_eq!(gpu_kind("intel iris xe graphics"), "integrated");
    assert_eq!(gpu_kind("amd radeon 780m graphics"), "integrated");
    assert_eq!(gpu_kind("amd radeon rx 7600"), "dedicated");
    assert_eq!(gpu_kind("nvidia rtx a2000 8gb laptop gpu"), "dedicated");
    assert_eq!(gpu_kind("qualcomm adreno x1-85 gpu"), "integrated");
}

#[test]
fn windows_corrections() {
    let f = windows::parse(&serde_json::json!({
        "serial": "PF5TSTVA", "chassis": [10],
        "memory": [{"Capacity": 34359738368u64, "SMBIOSMemoryType": 35, "FormFactor": 12, "ConfiguredClockSpeed": 8533}],
        "gpus": [{"Name": "Intel(R) Arc(TM) 140V GPU (16GB)", "AdapterRAM": 4293918720u64}],
        "disks": [{"Size": 1024209543168u64, "MediaType": "SSD", "BusType": "RAID", "Model": "NVMe SKHynix_HFS001TEJ9X115N"}],
        "nics": [
            {"Description": "Intel(R) Wi-Fi 7 BE201 320MHz", "MacAddress": "9C-65-EB-9F-18-E3", "Media": "Native 802.11", "Pnp": "PCI\\VEN_8086&DEV_A840"},
            {"Description": "Realtek USB GbE Family Controller", "MacAddress": "D0-C0-BF-28-E9-82", "Media": "802.3", "Pnp": "USB\\VID_0BDA&PID_8153\\001000001", "Speed": 1000000000u64}
        ],
        "bitlocker": 4,
        "battery": {"Full": 55000, "Design": 57000, "Present": true, "Admin": true}
    }));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.replaceable.as_str(), ram.modules, ram.form_factor.as_str()), (Some(32.0), "LPDDR5", "no", None, ""));
    assert_eq!((f.gpus[0].name.as_str(), f.gpus[0].kind.as_str(), f.gpus[0].vram_gb), ("Intel Arc 140V GPU", "integrated", None));
    assert_eq!((f.disks[0].gb, f.disks[0].connector.as_str(), f.disks[0].form_factor.as_str()), (1000.0, "NVMe", "M.2"));
    assert_eq!(f.nics.len(), 1, "the USB adapter is not the laptop's own card");
    assert_eq!(f.encryption, "no"); // decrypting
}

#[test]
fn linux_corrections() {
    let mut i = linux::Inputs::default();
    i.meminfo = "MemTotal:       15921532 kB\n".into();
    i.is_root = true;
    i.dmidecode_memory = "Memory Device\n\tSize: 8 GB\n\tForm Factor: SODIMM\n\tType: DDR4\n\tSpeed: 3200 MT/s\nMemory Device\n\tSize: 8 GB\n\tForm Factor: SODIMM\n\tType: DDR4\n\tSpeed: 3200 MT/s\n".into();
    i.nets = vec![
        linux::NetDevice { name: "enp0s31f6".into(), mac: "8c:8c:aa:dd:ee:ff".into(), wireless: false, physical: true, pci_slot: String::new(), usb: false },
        linux::NetDevice { name: "enx00e04c680001".into(), mac: "00:e0:4c:68:00:01".into(), wireless: false, physical: true, pci_slot: String::new(), usb: true },
    ];
    let f = linux::parse(&i);
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.modules, ram.module_gb, ram.form_factor.as_str()), (Some(16.0), Some(2), Some(8.0), "SO-DIMM"));
    assert_eq!(f.nics.len(), 1, "the USB adapter is left out");
}

#[test]
fn linux_encryption_is_about_the_system_drive() {
    let parse = |blockdevices: serde_json::Value| {
        let mut i = linux::Inputs::default();
        i.lsblk = serde_json::json!({ "blockdevices": blockdevices });
        linux::parse(&i).encryption
    };
    // "/" inside LUKS: encrypted
    assert_eq!(parse(serde_json::json!([{"name": "nvme0n1", "size": 512110190592u64, "type": "disk", "rm": false, "children": [
        {"name": "nvme0n1p2", "type": "part", "children": [{"name": "luks-1", "type": "crypt", "children": [{"name": "vg-root", "type": "lvm", "mountpoint": "/"}]}]}]}])), "yes");
    // "/" plain, a second drive encrypted: not encrypted
    assert_eq!(parse(serde_json::json!([
        {"name": "nvme0n1", "size": 512110190592u64, "type": "disk", "rm": false, "children": [{"name": "nvme0n1p2", "type": "part", "mountpoint": "/"}]},
        {"name": "sda", "size": 1000204886016u64, "type": "disk", "rm": false, "children": [{"name": "data", "type": "crypt", "mountpoint": "/data"}]}])), "no");
}

#[test]
fn windows_edition_in_any_language() {
    let os = |caption: &str, edition: &str, build: &str| {
        windows::parse(&serde_json::json!({ "serial": "X", "os": {"Caption": caption, "EditionID": edition, "DisplayVersion": "24H2", "Build": build} })).os.unwrap()
    };
    let spanish = os("Microsoft Windows 11 Profesional", "Professional", "26100");
    assert_eq!((spanish.version.as_str(), spanish.edition.as_str()), ("11 24H2", "Pro"));
    assert_eq!(os("Microsoft Windows 11 Home", "Core", "26100").edition, "Home");
    assert_eq!(os("Microsoft Windows 10 Pro", "Professional", "19045").version, "10 24H2");
    assert_eq!(os("Microsoft Windows 10 Pro", "Professional", "22631").version, "11 24H2"); // the build wins
    assert_eq!(os("Microsoft Windows 11 Pro", "", "26100").edition, "Pro"); // no code: from the description
}

#[test]
fn mac_pcie_ssd_listed_with_sata() {
    let profiler = serde_json::json!({
        "SPHardwareDataType": [{"machine_model": "MacBookPro11,1", "machine_name": "MacBook Pro"}],
        "SPSerialATADataType": [{"_name": "Apple SSD Controller", "spsata_negotiatedlinkspeed": "5.0 GT/s",
            "_items": [{"_name": "APPLE SSD SM0256F", "size_in_bytes": 251000193024u64}]}]
    });
    let f = macos::parse(&macos::Inputs {
        profiler, cpu_brand: "Intel(R) Core(TM) i5-4258U CPU @ 2.40GHz".into(), product_version: String::new(), hardware_ports: String::new(),
        filevault: String::new(), directory: None, ioreg_battery: String::new(), hostname: String::new(), screen_mm: None,
        nvram_oclp: String::new(), nvram_opencore: String::new(), oclp_patches: String::new(), media: Default::default(),
    });
    let d = &f.disks[0];
    assert_eq!((d.gb, d.kind.as_str(), d.connector.as_str(), d.form_factor.as_str(), d.replaceable.as_str()), (256.0, "SSD", "PCIe 2.0", "Apple (propietario)", "yes"));
}

/// What the Windows script itself printed (run in PowerShell 7 with stand-ins for the Windows-only commands that
/// answer like a ThinkPad X1 Carbon Gen 13 with a USB Ethernet adapter), read the way the app reads it.
#[test]
fn windows_script_output_end_to_end() {
    let json: serde_json::Value = serde_json::from_str(include_str!("windows_script_output.json")).unwrap();
    let mut f = windows::parse(&json);
    collect::tidy(&mut f);
    assert_eq!((f.manufacturer.as_str(), f.model.as_str(), f.chassis.as_str(), f.serial.as_str()), ("Lenovo", "ThinkPad X1 Carbon Gen 13", "laptop", "PF5TSTVA"));
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!((cpu.name.as_str(), cpu.cores, cpu.max_ghz), ("Intel Core Ultra 7 258V", Some(8), Some(2.2)));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.replaceable.as_str()), (Some(32.0), "LPDDR5", Some(8533.0), "no"));
    let gpus: Vec<(&str, &str, Option<f64>)> = f.gpus.iter().map(|g| (g.name.as_str(), g.kind.as_str(), g.vram_gb)).collect();
    assert_eq!(gpus, vec![("Intel Arc 140V GPU", "integrated", None), ("NVIDIA RTX A1000 6GB Laptop GPU", "dedicated", Some(6.0))]);
    let d = &f.disks[0];
    assert_eq!((d.gb, d.kind.as_str(), d.connector.as_str(), d.replaceable.as_str(), d.form_factor.as_str()), (1000.0, "SSD", "NVMe", "yes", "M.2"));
    assert_eq!(f.screen_inches, Some(14.0));
    assert_eq!(f.battery.as_ref().unwrap().health_pct, 97.0);
    assert_eq!(f.battery_cycles, Some(37)); // from the battery report: WMI said 0
    assert_eq!((d.health.as_str(), d.wear_pct, d.hours), ("ok", Some(3.0), Some(412)));
    let os = f.os.as_ref().unwrap();
    assert_eq!((os.version.as_str(), os.edition.as_str()), ("11 25H2", "Pro"));
    assert_eq!(f.nics.len(), 1, "only the built-in Wi-Fi: the USB adapter is left out");
    assert_eq!((f.nics[0].wifi_gen.as_str(), f.nics[0].bt_ver.as_str()), ("7", "5.4"));
    assert!(f.warnings.is_empty(), "{:?}", f.warnings);
}

#[test]
fn drive_health_and_battery_cycles() {
    use inventario_core::collect::util::disk_health;
    assert_eq!((disk_health("Verified"), disk_health("Failing"), disk_health("Not Supported")), ("ok", "failing", ""));
    assert_eq!((disk_health("Healthy"), disk_health("Warning"), disk_health("Unhealthy"), disk_health("2")), ("ok", "warning", "failing", "failing"));

    // Linux: smartctl -j -H -A
    let nvme = serde_json::json!({"smart_status": {"passed": true}, "power_on_time": {"hours": 1200},
        "nvme_smart_health_information_log": {"critical_warning": 0, "percentage_used": 7, "power_on_hours": 1200}});
    assert_eq!(linux::smart_health(&nvme), ("ok".to_string(), Some(7.0), Some(1200)));
    let worn = serde_json::json!({"smart_status": {"passed": true}, "nvme_smart_health_information_log": {"critical_warning": 1, "percentage_used": 104}});
    assert_eq!(linux::smart_health(&worn).0, "warning");
    let sata = serde_json::json!({"smart_status": {"passed": false}, "power_on_time": {"hours": 30500},
        "ata_smart_attributes": {"table": [{"id": 9, "value": 65}, {"id": 231, "value": 88}]}});
    assert_eq!(linux::smart_health(&sata), ("failing".to_string(), Some(12.0), Some(30500)));
    assert_eq!(linux::smart_health(&serde_json::json!({})), (String::new(), None, None));

    // macOS: smart_status and the cycle count
    let profiler = serde_json::json!({
        "SPHardwareDataType": [{"machine_model": "MacBookPro11,1", "machine_name": "MacBook Pro"}],
        "SPNVMeDataType": [{"_items": [{"_name": "APPLE SSD AP0256", "size_in_bytes": 251000193024u64, "smart_status": "Verified"}]}],
        "SPPowerDataType": [{"sppower_battery_health_info": {"sppower_battery_health_maximum_capacity": "81%", "sppower_battery_cycle_count": 612}}]
    });
    let f = macos::parse(&macos::Inputs {
        profiler, cpu_brand: "Intel(R) Core(TM) i5-4258U CPU @ 2.40GHz".into(), product_version: String::new(), hardware_ports: String::new(),
        filevault: String::new(), directory: None, ioreg_battery: String::new(), hostname: String::new(), screen_mm: None,
        nvram_oclp: String::new(), nvram_opencore: String::new(), oclp_patches: String::new(), media: Default::default(),
    });
    assert_eq!((f.disks[0].health.as_str(), f.battery_cycles), ("ok", Some(612)));

    // Linux battery cycles come straight from /sys
    let mut i = linux::Inputs::default();
    i.battery = Some((45.0, 50.0));
    i.battery_cycles = Some(230);
    assert_eq!(linux::parse(&i).battery_cycles, Some(230));
}

#[test]
fn run_timeout_reads_long_output_and_stops_hung_programs() {
    use inventario_core::collect::util::run_timeout;
    // 200 KB of output: more than a pipe holds, so it must be read while the program runs
    let out = run_timeout("sh", &["-c", "head -c 200000 /dev/zero | tr '\\0' x"], 10).unwrap();
    assert_eq!(out.len(), 200000);
    let started = std::time::Instant::now();
    assert_eq!(run_timeout("sleep", &["30"], 1), None);
    assert!(started.elapsed().as_secs() < 5);
    assert_eq!(run_timeout("this-program-does-not-exist", &[], 1), None);
}

#[test]
fn mac_ethernet_chip_across_macos_versions() {
    use macos::{ethernet_chip, ethernet_entry};
    let mac = "00:1C:42:AB:CD:01";
    // newer macOS: BSD name field
    let p = json!({"SPEthernetDataType": [{"_name": "ethernet", "spethernet_BSD_Device_Name": "en0",
        "spethernet_vendor-id": "0x14e4", "spethernet_device-id": "0x16b4", "spethernet_subsystem-vendor-id": "0x106b"}]});
    assert_eq!(ethernet_chip(&ethernet_entry(&p, "en0", mac, 1).unwrap()), "Broadcom BCM57765");
    // another BSD spelling, nested under _items, upper-case ids
    let p = json!({"SPEthernetDataType": [{"_items": [{"_name": "en0", "spethernet_bsd_name": "en0",
        "spethernet_vendor_id": "0x14E4", "spethernet_device_id": "0x16B4"}]}]});
    assert_eq!(ethernet_chip(&ethernet_entry(&p, "en0", mac, 1).unwrap()), "Broadcom BCM57765");
    // no BSD name, found by MAC address
    let p = json!({"SPEthernetDataType": [
        {"_name": "Ethernet", "spethernet_mac-address": "aa:bb:cc:dd:ee:ff", "spethernet_vendor-id": "0x8086", "spethernet_device-id": "0x1533"},
        {"_name": "Ethernet", "spethernet_mac-address": "00-1c-42-ab-cd-01", "spethernet_vendor-id": "0x14e4", "spethernet_device-id": "0x16b4"}]});
    assert_eq!(ethernet_chip(&ethernet_entry(&p, "en0", mac, 2).unwrap()), "Broadcom BCM57765");
    // no BSD name nor MAC: the only entry of a Mac with one port
    let p = json!({"SPEthernetDataType": [{"_name": "ethernet", "spethernet_vendor-id": "0x14e4", "spethernet_device-id": "0x16b4"}]});
    assert_eq!(ethernet_chip(&ethernet_entry(&p, "en0", mac, 1).unwrap()), "Broadcom BCM57765");
    assert!(ethernet_entry(&p, "en0", mac, 2).is_none(), "two ports, one entry: unknown which");
    // an entry naming another device is never used as a fallback
    let p = json!({"SPEthernetDataType": [{"spethernet_BSD_Device_Name": "en5", "spethernet_vendor-id": "0x14e4"}]});
    assert!(ethernet_entry(&p, "en0", mac, 1).is_none());
    // vendor given by name; unknown device id
    assert_eq!(ethernet_chip(&json!({"_name": "ethernet", "spethernet_vendor": "Broadcom Limited"})), "Broadcom Ethernet");
    // a chip name macOS gives is kept, a BSD name is not taken as one
    assert_eq!(ethernet_chip(&json!({"_name": "Aquantia AQC113"})), "Aquantia AQC113");
    assert_eq!(ethernet_chip(&json!({"_name": "en0"})), "");
    assert_eq!(ethernet_chip(&json!({"_name": "ethernet"})), "");
}

/// What macOS 10.15 Catalina printed on an iMac14,2 (late 2013, 27"), serials and MAC addresses replaced.
#[test]
fn mac_catalina_imac() {
    let profiler: serde_json::Value = serde_json::from_str(include_str!("macos_catalina_imac.json")).unwrap();
    let mut media = HashMap::new();
    media.insert("en0".to_string(), "\tmedia: 1000baseT <full-duplex> (none)\n\tstatus: inactive\n\tsupported media:\n\t\tmedia none\n\t\tmedia autoselect\n\t\tmedia 10baseT/UTP mediaopt full-duplex\n\t\tmedia 100baseTX mediaopt full-duplex\n\t\tmedia 1000baseT mediaopt full-duplex\n".to_string());
    let f = macos::parse(&macos::Inputs {
        profiler, cpu_brand: "Intel(R) Core(TM) i5-4670 CPU @ 3.40GHz".into(), product_version: "10.15.7".into(),
        hardware_ports: "Hardware Port: Ethernet\nDevice: en0\nEthernet Address: 00:1c:42:00:00:01\n\nHardware Port: Wi-Fi\nDevice: en1\nEthernet Address: 00:1c:42:00:00:02\n\nHardware Port: Bluetooth PAN\nDevice: en4\nEthernet Address: 00:1c:42:00:00:03\n\nHardware Port: Thunderbolt 1\nDevice: en2\nEthernet Address: 82:0d:46:00:00:80\n\nHardware Port: Thunderbolt Bridge\nDevice: bridge0\nEthernet Address: 82:0d:46:00:00:80\n".into(),
        filevault: "FileVault is Off.".into(), directory: Some(String::new()), ioreg_battery: String::new(), hostname: "ZIMC035".into(),
        screen_mm: Some((597.0, 336.0)), nvram_oclp: String::new(), nvram_opencore: String::new(), oclp_patches: String::new(), media,
    });
    assert_eq!((f.model.as_str(), f.chassis.as_str(), f.screen_inches), ("iMac (iMac14,2)", "all_in_one", Some(27.0)));
    assert_eq!(f.cpu.as_ref().map(|c| (c.name.as_str(), c.cores)), Some(("Intel Core i5-4670", Some(4))));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.modules, ram.module_gb, ram.form_factor.as_str(), ram.replaceable.as_str()),
               (Some(16.0), "DDR3", Some(1600.0), Some(4), Some(4.0), "SO-DIMM", "yes"));
    let d = &f.disks[0];
    assert_eq!((f.disks.len(), d.gb, d.kind.as_str(), d.form_factor.as_str(), d.replaceable.as_str(), d.health.as_str()), (1, 1000.0, "HDD", "3.5\"", "yes", "ok"));
    assert_eq!(f.gpus.iter().map(|g| (g.name.as_str(), g.kind.as_str(), g.vram_gb)).collect::<Vec<_>>(), [("NVIDIA GeForce GTX 775M", "dedicated", Some(2.0))]);
    assert_eq!(f.os.as_ref().map(|o| (o.version.as_str(), o.number.as_str())), Some(("10.15 Catalina", "10.15.7")));
    assert_eq!((f.encryption.as_str(), f.domain_joined.as_str(), f.battery.is_none()), ("no", "no", true));
    assert_eq!(f.nics.len(), 2, "the built-in Ethernet and Wi-Fi only");
    assert_eq!(f.nics.iter().find(|n| n.kind == "wifi").map(|n| n.wifi_gen.as_str()), Some("5"));
    let eth = f.nics.iter().find(|n| n.kind == "eth").unwrap();
    assert_eq!((eth.model.as_str(), eth.speed_mbps), ("Broadcom 57766-A1", Some(1000)));
}
