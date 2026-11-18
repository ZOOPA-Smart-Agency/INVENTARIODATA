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
    });
    collect::tidy(&mut f);
    assert_eq!(f.serial, "C02XK3JHMD6R");
    assert_eq!(f.model, "MacBook Pro (MacBookPro15,1)");
    assert_eq!(f.chassis, "laptop");
    let cpu = f.cpu.as_ref().unwrap();
    assert_eq!((cpu.name.as_str(), cpu.cores, cpu.max_ghz), ("Intel Core i7-8850H", Some(6), Some(2.6)));
    let ram = f.ram.as_ref().unwrap();
    assert_eq!((ram.gb, ram.kind.as_str(), ram.mts, ram.replaceable.as_str()), (Some(16.0), "DDR4", Some(2400.0), "no"));
    assert_eq!((f.disks[0].gb, f.disks[0].connector.as_str(), f.disks[0].replaceable.as_str()), (500.0, "NVMe", "no")); // a T2 Mac
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
    assert_eq!((f.nics[0].model.as_str(), f.nics[0].wifi_gen.as_str()), ("Broadcom BCM7BF", "5"));
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
        lspci: "00:02.0 \"VGA compatible controller\" \"Intel Corporation\" \"Raptor Lake-P [Iris Xe Graphics]\" -r04 \"Dell\" \"Device 0c0d\"\n00:14.3 \"Network controller\" \"Intel Corporation\" \"Raptor Lake PCH CNVi WiFi\" -r01 \"Intel Corporation\" \"Wi-Fi 6E AX211 160MHz\"\n00:1f.6 \"Ethernet controller\" \"Intel Corporation\" \"Ethernet Connection (16) I219-LM\" -r01 \"Dell\" \"Device 0c0d\"\n".into(),
        nets: vec![
            linux::NetDevice { name: "lo".into(), mac: "00:00:00:00:00:00".into(), wireless: false, physical: false, pci_slot: String::new() },
            linux::NetDevice { name: "wlp0s20f3".into(), mac: "a4:c3:f0:aa:bb:cc".into(), wireless: true, physical: true, pci_slot: "0000:00:14.3".into() },
            linux::NetDevice { name: "enp0s31f6".into(), mac: "8c:8c:aa:dd:ee:ff".into(), wireless: false, physical: true, pci_slot: "0000:00:1f.6".into() },
            linux::NetDevice { name: "docker0".into(), mac: "02:42:ac:11:00:01".into(), wireless: false, physical: false, pci_slot: String::new() },
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
            nvram_oclp: oclp.into(), nvram_opencore: opencore.into(), oclp_patches: patches.into(),
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
        });
        f.ram.unwrap().form_factor
    };
    assert_eq!(mac("iMac14,2", "iMac"), "SO-DIMM");
    assert_eq!(mac("Macmini7,1", "Mac mini"), "SO-DIMM");
    assert_eq!(mac("MacBookPro9,2", "MacBook Pro"), "SO-DIMM");
    assert_eq!(mac("MacPro5,1", "Mac Pro"), "DIMM");
}
