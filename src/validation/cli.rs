use clap::{App, Arg};

pub fn cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("validator_client")
        .visible_aliases(&["v", "vc", "validator"])
        .setting(clap::AppSettings::ColoredHelp)
        .about(
            "When connected to a beacon node, performs the duties of a staked \
                validator (e.g., proposing blocks and attestations).",
        )
        // This argument is deprecated, use `--beacon-nodes` instead.
        .arg(
            Arg::with_name("beacon-node")
                .long("beacon-node")
                .value_name("NETWORK_ADDRESS")
                .help("Deprecated. Use --beacon-nodes.")
                .takes_value(true)
                .conflicts_with("beacon-nodes"),
        )
        .arg(
            Arg::with_name("beacon-nodes")
                .long("beacon-nodes")
                .value_name("NETWORK_ADDRESSES")
                .help("Comma-separated addresses to one or more beacon node HTTP APIs. \
                       Default is http://localhost:5052."
                )
                .takes_value(true),
        )
        // This argument is deprecated, use `--beacon-nodes` instead.
        .arg(
            Arg::with_name("server")
                .long("server")
                .value_name("NETWORK_ADDRESS")
                .help("Deprecated. Use --beacon-nodes.")
                .takes_value(true)
                .conflicts_with_all(&["beacon-node", "beacon-nodes"]),
        )
        .arg(
            Arg::with_name("validators-dir")
                .long("validators-dir")
                .alias("validator-dir")
                .value_name("VALIDATORS_DIR")
                .help(
                    "The directory which contains the validator keystores, deposit data for \
                    each validator along with the common slashing protection database \
                    and the validator_definitions.yml"
                )
                .takes_value(true)
                .conflicts_with("datadir")
        )
        .arg(
            Arg::with_name("secrets-dir")
                .long("secrets-dir")
                .value_name("SECRETS_DIRECTORY")
                .help(
                    "The directory which contains the password to unlock the validator \
                    voting keypairs. Each password should be contained in a file where the \
                    name is the 0x-prefixed hex representation of the validators voting public \
                    key. Defaults to ~/.lighthouse/{network}/secrets.",
                )
                .takes_value(true)
                .conflicts_with("datadir")
        )
        .arg(
            Arg::with_name("ip")
                .long("ip")
                .value_name("NODE_IP")
                .help(
                    "This node's ip which is used for connections with other nodes"
                )
                .takes_value(true)
        )
        .arg(
            Arg::with_name("id")
            .long("id")
            .value_name("NODE_ID")
            .help(
                "This node's id in smart contract"
            )
            .takes_value(true)
        )
        .arg(
            Arg::with_name("api")
                .long("api")
                .value_name("API")
                .help(
                    "The api where the operator to send performance request"
                ).takes_value(true)
        )
        .arg(
            Arg::with_name("base-port")
                .long("base-port")
                .value_name("BASE_PORT")
                .help(
                    "This node's BASE_PORT"
                )
                .takes_value(true)
        )
        .arg(
            Arg::with_name("ws-url")
            .long("ws-url")
            .value_name("WS_URL")
            .help("web socket url of infura to listen contract event")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("boot-enr")
                .long("boot-enr")
                .value_name("BOOT_ENR")
                .help(
                    "This is the enr of the root node"
                )
                .takes_value(true)
        )
        .arg(
            Arg::with_name("registry-contract")
                .long("registry-contract")
                .value_name("REGISTRY_CONTRACT")
                .help(
                    "This is the address of registry contract"
                )
                .takes_value(true)
        )
        .arg(
            Arg::with_name("network-contract")
                .long("network-contract")
                .value_name("NETWORK_CONTRACT")
                .help(
                    "This is the address of network contract"
                )
                .takes_value(true)
        )
        .arg(
            Arg::with_name("delete-lockfiles")
            .long("delete-lockfiles")
            .help(
                "DEPRECATED. This flag does nothing and will be removed in a future release."
            )
        )
        .arg(
            Arg::with_name("init-slashing-protection")
                .long("init-slashing-protection")
                .help(
                    "If present, do not require the slashing protection database to exist before \
                     running. You SHOULD NOT use this flag unless you're certain that a new \
                     slashing protection database is required. Usually, your database \
                     will have been initialized when you imported your validator keys. If you \
                     misplace your database and then run with this flag you risk being slashed."
                )
        )
        .arg(
            Arg::with_name("disable-auto-discover")
            .long("disable-auto-discover")
            .help(
                "If present, do not attempt to discover new validators in the validators-dir. Validators \
                will need to be manually added to the validator_definitions.yml file."
            )
        )
        .arg(
            Arg::with_name("allow-unsynced")
                .long("allow-unsynced")
                .help(
                    "If present, the validator client will still poll for duties if the beacon
                      node is not synced.",
                ),
        )
        .arg(
            Arg::with_name("use-long-timeouts")
                .long("use-long-timeouts")
                .help("If present, the validator client will use longer timeouts for requests \
                        made to the beacon node. This flag is generally not recommended, \
                        longer timeouts can cause missed duties when fallbacks are used.")
        )
        .arg(
            Arg::with_name("beacon-nodes-tls-certs")
                .long("beacon-nodes-tls-certs")
                .value_name("CERTIFICATE-FILES")
                .takes_value(true)
                .help("Comma-separated paths to custom TLS certificates to use when connecting \
                        to a beacon node. These certificates must be in PEM format and are used \
                        in addition to the OS trust store. Commas must only be used as a \
                        delimiter, and must not be part of the certificate path.")
        )
        // This overwrites the graffiti configured in the beacon node.
        .arg(
            Arg::with_name("graffiti")
                .long("graffiti")
                .help("Specify your custom graffiti to be included in blocks.")
                .value_name("GRAFFITI")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("graffiti-file")
                .long("graffiti-file")
                .help("Specify a graffiti file to load validator graffitis from.")
                .value_name("GRAFFITI-FILE")
                .takes_value(true)
                .conflicts_with("graffiti")
        )
        .arg(
            Arg::with_name("suggested-fee-recipient")
                .long("suggested-fee-recipient")
                .help("The fallback address provided to the BN if nothing suitable is found \
                           in the validator definitions or fee recipient file.")
                .value_name("FEE-RECIPIENT")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("suggested-fee-recipient-file")
                .long("suggested-fee-recipient-file")
                .help("The fallback address provided to the BN if nothing suitable is found \
                           in the validator definitions.")
                .value_name("FEE-RECIPIENT-FILE")
                .takes_value(true)
        )
        /* REST API related arguments */
        .arg(
            Arg::with_name("http")
                .long("http")
                .help("Enable the RESTful HTTP API server. Disabled by default.")
                .takes_value(false),
        )
        /*
         * Note: The HTTP server is **not** encrypted (i.e., not HTTPS) and therefore it is
         * unsafe to publish on a public network.
         *
         * If the `--http-address` flag is used, the `--unencrypted-http-transport` flag
         * must also be used in order to make it clear to the user that this is unsafe.
         */
         .arg(
             Arg::with_name("http-address")
                 .long("http-address")
                 .value_name("ADDRESS")
                 .help("Set the address for the HTTP address. The HTTP server is not encrypted \
                        and therefore it is unsafe to publish on a public network. When this \
                        flag is used, it additionally requires the explicit use of the \
                        `--unencrypted-http-transport` flag to ensure the user is aware of the \
                        risks involved. For access via the Internet, users should apply \
                        transport-layer security like a HTTPS reverse-proxy or SSH tunnelling.")
                .requires("unencrypted-http-transport"),
         )
         .arg(
             Arg::with_name("unencrypted-http-transport")
                 .long("unencrypted-http-transport")
                 .help("This is a safety flag to ensure that the user is aware that the http \
                        transport is unencrypted and using a custom HTTP address is unsafe.")
                 .requires("http-address"),
         )
        .arg(
            Arg::with_name("http-port")
                .long("http-port")
                .value_name("PORT")
                .help("Set the listen TCP port for the RESTful HTTP API server.")
                .default_value("5062")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("http-allow-origin")
                .long("http-allow-origin")
                .value_name("ORIGIN")
                .help("Set the value of the Access-Control-Allow-Origin response HTTP header. \
                    Use * to allow any origin (not recommended in production). \
                    If no value is supplied, the CORS allowed origin is set to the listen \
                    address of this server (e.g., http://localhost:5062).")
                .takes_value(true),
        )
        /* Prometheus metrics HTTP server related arguments */
        .arg(
            Arg::with_name("metrics")
                .long("metrics")
                .help("Enable the Prometheus metrics HTTP server. Disabled by default.")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("metrics-address")
                .long("metrics-address")
                .value_name("ADDRESS")
                .help("Set the listen address for the Prometheus metrics HTTP server.")
                .default_value("127.0.0.1")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("metrics-port")
                .long("metrics-port")
                .value_name("PORT")
                .help("Set the listen TCP port for the Prometheus metrics HTTP server.")
                .default_value("5064")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("metrics-allow-origin")
                .long("metrics-allow-origin")
                .value_name("ORIGIN")
                .help("Set the value of the Access-Control-Allow-Origin response HTTP header. \
                    Use * to allow any origin (not recommended in production). \
                    If no value is supplied, the CORS allowed origin is set to the listen \
                    address of this server (e.g., http://localhost:5064).")
                .takes_value(true),
        )
        /*
         * Explorer metrics
         */
         .arg(
            Arg::with_name("monitoring-endpoint")
                .long("monitoring-endpoint")
                .value_name("ADDRESS")
                .help("Enables the monitoring service for sending system metrics to a remote endpoint. \
                This can be used to monitor your setup on certain services (e.g. beaconcha.in). \
                This flag sets the endpoint where the beacon node metrics will be sent. \
                Note: This will send information to a remote sever which may identify and associate your \
                validators, IP address and other personal information. Always use a HTTPS connection \
                and never provide an untrusted URL.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("enable-doppelganger-protection")
                .long("enable-doppelganger-protection")
                .value_name("ENABLE_DOPPELGANGER_PROTECTION")
                .help("If this flag is set, Lighthouse will delay startup for three epochs and \
                    monitor for messages on the network by any of the validators managed by this \
                    client. This will result in three (possibly four) epochs worth of missed \
                    attestations. If an attestation is detected during this period, it means it is \
                    very likely that you are running a second validator client with the same keys. \
                    This validator client will immediately shutdown if this is detected in order \
                    to avoid potentially committing a slashable offense. Use this flag in order to \
                    ENABLE this functionality, without this flag Lighthouse will begin attesting \
                    immediately.")
                .takes_value(false),
        )
        // .arg(
        //     Arg::with_name("private-tx-proposals")
        //         .long("private-tx-proposals")
        //         .help("If this flag is set, Lighthouse will query the Beacon Node for only block \
        //             headers during proposals and will sign over headers. Useful for outsourcing \
        //             execution payload construction during proposals.")
        //         .takes_value(false),
        // )
        .arg(
            Arg::with_name("builder-proposals")
                .long("builder-proposals")
                .alias("private-tx-proposals")
                .help("If this flag is set, Lighthouse will query the Beacon Node for only block \
                    headers during proposals and will sign over headers. Useful for outsourcing \
                    execution payload construction during proposals.")
                .takes_value(false),
        ).arg(
        Arg::with_name("gas-limit")
            .long("gas-limit")
            .value_name("GAS_LIMIT_INTEGER")
            .takes_value(true)
            .help("The gas limit to be used in all builder proposals for all validators managed \
                    by this validator client. Note this will not necessarily be used if the gas limit \
                    set here moves too far from the previous block's gas limit. [default: 30,000,000]")
            .requires("builder-proposals"),
        )
}
