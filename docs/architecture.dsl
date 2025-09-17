workspace "AKD Watch System" {

  !identifiers hierarchical

  model {
    properties {
      "structurizr.groupSeparator" "/"
    }

    client = softwareSystem "Client" {
      description "The client that depends on the AKD Watch to audit an AKD"
      tags "Client"
    }

    akd = softwareSystem "AKD" {
      tags "External"
      description "The AKD that is being audited"
    }

    namespace_storage = softwareSystem "Namespace Storage" {
      description "Stores the state of each AKD namespace being audited"
    }

    signature_storage = softwareSystem "Signature Storage" {
      description "Stores signatures of audit proofs, publicly accessible"
    }

    key_storage = softwareSystem "Key Storage" {
      description "Stores the private key used to sign audit proofs"
    }

    akd_watch = softwareSystem "AKD Watch" {
      description "The AKD Watch system that audits the AKD"

      auditor = container "Auditor" {
        description "The component that polls the AKD for new proofs, verifies the, and signs verified proofs"
        technology "Rust and reqwest"

        app = component "Application" {
          description "The main application thread handler for the auditor"
          technology "Rust"
        }

        namespace_auditor = component "Namespace Auditor" {
          description "Handles auditing a single AKD namespace"
          technology "Rust"
        }

        app -> namespace_auditor "Creates and manages namespace auditors" "tokio"
        namespace_auditor -> akd "Polls for audit proofs" "http"
        namespace_auditor -> signature_storage "Writes audit signatures" "filesystem" {
          tags "write"
        }
        namespace_auditor -> namespace_storage "Manages AKD namespace state" "filesystem" {
          tags "write"
        }
        app -> key_storage "Manages signing and verifying keys" "filesystem" {
          tags "write"
        }
      }

      web = container "Web" {
        description "The web interface for AKD Watch"
        technology "Rust with Axum"

        info = component "Info Endpoint" {
          description "Provides verifying key information about the AKD Watch instance"
          technology "Rust with Axum"
        }

        audits = component "Audit Endpoint" {
          description "Returns signed audits for a given AKD (namespace) and epoch"
          technology "Rust with Axum"
        }

        namespaces = component "Namespaces Endpoint" {
          description "Lists namespaces being audited with status information for each"
          technology "Rust with Axum"
        }

        namespace = component "Namespace Component" {
          description "Returns a single namespace's information"
          technology "Rust with Axum"
        }

        info -> key_storage "Reads verifying keys" "filesystem" {
          tags "readonly"
        }
        audits -> signature_storage "Reads audit signatures" "filesystem" {
          tags "readonly"
        }
        namespaces -> namespace_storage "Reads AKD namespace state" "filesystem" {
          tags "readonly"
        }
        namespace -> namespace_storage "Reads AKD namespace state" "filesystem" {
          tags "readonly"
        }
        client -> web "Requests and validates audit signatures for required epochs" "http"
        client -> info "Requests verifying key data" "http"
        client -> audits "Requests audit signatures" "http"
        client -> namespaces "Requests list of namespaces being audited" "http"
        client -> namespace "Requests information about a specific namespace" "http"
      }

      config = container "Configuration" {
        description "Application configuration data"
        !docs "../CONFIGURATION.md"
        technology "JSON"
        tags "Config"
      }
      config -> web "Reads configuration" "environment" "environment"
      config -> auditor "Reads configuration" "environment" "environment"
    }

    client -> akd "Requests configuration information to use in signature validation" "http"
    client -> signature_storage "May directly request signatures if storage is public" "http"
  }

  views {
    systemLandscape "akd_watch" {
      include *
      // autolayout tb
    }

    container akd_watch "akd_watch_server" {
      include *
      // autolayout tb
    }

    container akd_watch "akd_watch_web" {
      include *
      exclude akd_watch.auditor
      // autolayout tb
    }

    container akd_watch "akd_auditor" {
      include *
      exclude akd_watch.web
      exclude client
      // autolayout tb
    }

    component akd_watch.web "akd_watch_web_components" {
      include *
      // autolayout tb
    }

    component akd_watch.auditor "akd_watch_auditor_components" {
      include *
      autolayout tb
    }

    styles {
      theme default
      element "Element" {
        color #3c3b3b
      }
      element "Person" {
        background #d34407
        shape person
      }
      element "Queue" {
        shape pipe
      }
      element "Client" {
        shape mobileDevicePortrait
      }
      element "Web" {
        shape webBrowser
      }
      element "Database" {
        shape cylinder
      }
      element "Blob Storage" {
        shape cylinder
        background #08ac9c
      }
      element "Config" {
        shape folder
        background #685959
      }
      // Indicates a software system that is external to the AKD Watch system
      element "External" {
        color #000000
        background #b5b5b5
      }

      relationship "environment" {
        style solid
      }

      relationship "readonly" {
        color blue
      }

      relationship "write" {
        color red
      }
    }
  }
}
