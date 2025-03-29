$version: "2"

namespace org.typesofants

@pattern("^\\d\\d\\d$")
@length(min: 3, max: 3)
string HostNum

@pattern("^[a-zA-Z0-9-]+$")
@length(min: 64, max: 64)
string HostId

resource Host {
    identifiers: {
        hostNum: HostNum
        hostId: HostId
    }
    properties: {
    }
}

list ProjectList {
    member: Project
}

structure Project {
    name: String
}
