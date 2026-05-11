// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "PatchbayWorker",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "patchbay-worker",
            path: "Sources"
        )
    ]
)
