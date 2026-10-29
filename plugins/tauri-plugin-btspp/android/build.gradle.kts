plugins {
    id("com.android.library")
    kotlin("android")
}

android {
    namespace = "com.recuscan.btspp"
    compileSdk = 34
    defaultConfig {
        minSdk = 24
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

repositories {
    google()
    mavenCentral()
}

dependencies {
    // TODO: синхронизировать версию с tauri в src-tauri/Cargo.toml
    implementation("app.tauri:tauri-android:2.+")
}
