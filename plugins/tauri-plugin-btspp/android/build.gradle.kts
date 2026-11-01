import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library")
    kotlin("android")
}

android {
    namespace = "com.recuscan.btspp"
    compileSdk = 37
    defaultConfig {
        minSdk = 24
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        // Kotlin по умолчанию наследует таргет запущенной JVM (21) — выравниваем с Java 17
        jvmTarget = JvmTarget.JVM_17
    }
}

repositories {
    google()
    mavenCentral()
}

dependencies {
    // tauri-android не публикуется в Maven — Tauri CLI подключает его как локальный
    // Gradle-проект из cargo-реестра (см. gen/android/tauri.settings.gradle).
    implementation(project(":tauri-android"))
    // ActivityResult для @ActivityCallback (SAF-пикеры): androidx.* у tauri-android
    // объявлен как implementation и в compile classpath плагина не попадает
    implementation("androidx.activity:activity:1.9.3")
}
