// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "ParakeetWorker",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(path: "/Users/rocketman/crystalballmini/parakeet-coreml-swift")
    ],
    targets: [
        .executableTarget(
            name: "parakeet-worker",
            dependencies: [
                .product(name: "ParakeetTDT", package: "parakeet-coreml-swift")
            ],
            path: "Sources"
        )
    ]
)
