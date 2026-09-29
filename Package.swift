// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Kvoteven",
    platforms: [.macOS(.v13)],
    products: [.library(name: "QuotaCore", targets: ["QuotaCore"]),
               .executable(name: "Kvoteven", targets: ["Kvoteven"])],
    targets: [
        .target(name: "QuotaCore"),
        .executableTarget(name: "Kvoteven", dependencies: ["QuotaCore"]),
        .testTarget(name: "QuotaCoreTests", dependencies: ["QuotaCore"])
    ]
)
