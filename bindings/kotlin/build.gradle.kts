// JVM test of the generated Kotlin bindings (the same code an Android app uses).
// Generate first: see ../README.md. Run: gradle test -PlibDir=../target/release
plugins {
    kotlin("jvm") version "2.2.20"
}

repositories { mavenCentral() }

dependencies {
    implementation("net.java.dev.jna:jna:5.17.0")
    testImplementation(kotlin("test"))
}

kotlin {
    sourceSets["main"].kotlin.srcDir("generated")
}

tasks.test {
    useJUnitPlatform()
    systemProperty("jna.library.path", file(project.findProperty("libDir") ?: "../target/release").absolutePath)
    systemProperty("voxtrust.root", rootDir.resolve("../..").absolutePath)
}
