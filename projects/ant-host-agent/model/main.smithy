$version: "2"

namespace org.typesofants

use aws.protocols#restJson1

/// Service definition.
@title("A Service")
@restJson1
service AntHostAgent {
    version: "2024-03-02"
    resources: [
        Host
    ]
    operations: [
        Echo
    ]
}

@readonly
@http(uri: "/ping", method: "GET")
operation Echo {
    input := {}
    output := {
        response: String
    }
    errors: [
        InternalServerFailure
    ]
}

@error("server")
structure InternalServerFailure {
  message: String
}