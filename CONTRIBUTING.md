# Contributing

Thank you for taking the time to contribute to this repository! Contributions
are very welcome. The project aims to maximise developer experience, so
documentation feedback is especially valuable. Before opening a pull request,
please open an issue or reach out to @gfusee on GitHub to discuss the change.

## Running the tests

1. Build the Docker images used by the test suite:
   `./build_test_images.sh`
2. Run the tests: `cargo test`

The integration tests create Docker containers for the Sui localnet and Seal
servers. The suite cleans them up automatically when tests finish, so do not
stop them manually.
