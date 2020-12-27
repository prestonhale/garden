fn main()  {
    let config = garden::Config::new();
    println!("Running with host address: {}", config.host_address);

    garden::run(config);
}