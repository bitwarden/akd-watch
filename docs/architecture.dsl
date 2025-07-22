workspace "AKD Watch System" {

  !identifiers hierarchical

  model {
    properties {
      "structurizr.groupSeparator" "/"
    }

    akd_watch = softwareSystem "AKD Watch" {
      description "The AKD Watch system that audits the AKD"

      auditor = container "Auditor" {
        description "The component that audits the AKD"
        technology "Rust"
      }

      watcher = container "Watcher" {
        description "The component that watches the AKD for new published audit proofs"
        technology "Rust"
      }

      web = container "Web" {
        description "The web interface for AKD Watch"
        technology "Rust with Axum"
      }

      audit_queue = container "Audit Request Queue" {
        description "fault-tolerant queue to store audit requests"
        tags "Queue"
        // technology "Azure Queue or relational database-mediated queue"
      }

      database = container "Database" {
        description "Stores AKD epoch and configuration information"
        technology "PostgreSQL"
        tags "Database"
      }

      config = container "Configuration" {
        description "Connection stings, keys, AKD namespaces and other configuration information"
        technology "JSON"
        tags "Config"
      }
    }

    client = softwareSystem "Client" {
      description "The client that depends on the AKD Watch to audit an AKD"
      tags "Client"
    }

    akd = softwareSystem "AKD" {
      tags "External"
      description "The AKD that is being audited"
    }

    signature_storage = softwareSystem "Signature Storage" {
      description "Stores signatures of audit proofs, publicly accessible"
      tags "Blob Storage"
    }


    client -> akd_watch.web "Requests and validates audit signatures for required epochs" "http"
    client -> akd "Requests and validates lookup and history proofs" "http"
    client -> signature_storage "Requests and validates audit signatures" "http"

    akd_watch.watcher -> akd "Poll for audit proofs" "http"
    akd_watch.auditor -> akd "Downloads and audits epoch proofs" "http"

    // akd_watch.config -> akd_watch.web "Reads configuration" "environment" "environment"
    // akd_watch.config -> akd_watch.auditor "Reads configuration" "environment" "environment"
    // akd_watch.config -> akd_watch.watcher "Reads configuration" "environment" "environment"

    akd_watch.database -> akd_watch.web "Reads AKD namespace state" {
      tags "readonly"
    }
    akd_watch.database -> akd_watch.watcher "Reads AKD namespace state" {
      tags "readonly"
    }
    akd_watch.auditor -> akd_watch.database "Writes updates to AKD namespace state" {
      tags "write"
    }

    akd_watch.auditor -> signature_storage "Stores audit signatures"
    akd_watch.web -> signature_storage "reads audit signatures"

    akd_watch.watcher -> akd_watch.audit_queue "Enqueue new audit proofs to be audited"
    akd_watch.audit_queue -> akd_watch.auditor "Dequeue audit proofs to be audited"

    akd_watch.web -> akd_watch.audit_queue "Enqueue missing audit signatures"
  }

  views {
    systemLandscape "akd_watch" {
      include *
    }

    container akd_watch "akd_watch_server" {
      include *
    }

    container akd_watch "akd_watch_web" {
      include *
      exclude akd_watch.auditor
      exclude akd_watch.watcher
    }

    container akd_watch "akd_watch_auditor" {
      include *
      exclude akd_watch.web
      exclude akd_watch.watcher
    }

    container akd_watch "akd_watch_watcher" {
      include *
      exclude akd_watch.web
      exclude akd_watch.auditor
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
