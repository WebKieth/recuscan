package com.recuscan.btspp

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.IOException
import java.util.UUID

/**
 * Bluetooth Classic SPP (RFCOMM) канал до ELM327-адаптера.
 * Адаптер должен быть предварительно сопряжён в настройках Android.
 *
 * TODO(on-device): проверить поток запроса разрешений на реальном устройстве
 * (requestPermissionForAlias + *Callback) — API tauri-android периодически меняется.
 */
@SuppressLint("MissingPermission") // разрешения проверяются в ensureBluetoothPermission
@TauriPlugin(
    permissions = [
        Permission(
            strings = [Manifest.permission.BLUETOOTH_CONNECT],
            alias = "bluetoothConnect"
        )
    ]
)
class BtSppPlugin(private val activity: Activity) : Plugin(activity) {
    // Стандартный UUID сервиса Serial Port Profile
    private val sppUuid: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")

    private var socket: BluetoothSocket? = null

    private fun adapter(): BluetoothAdapter? {
        val manager = activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
        return manager?.adapter
    }

    /** Запрашивает BLUETOOTH_CONNECT на API 31+, иначе возвращает true сразу. */
    private fun ensureBluetoothPermission(invoke: Invoke, callback: String): Boolean {
        val granted = Build.VERSION.SDK_INT < 31 ||
            activity.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) ==
                PackageManager.PERMISSION_GRANTED
        if (!granted) {
            requestPermissionForAlias("bluetoothConnect", invoke, callback)
        }
        return granted
    }

    @Command
    fun listPaired(invoke: Invoke) {
        if (!ensureBluetoothPermission(invoke, "listPairedCallback")) return
        doListPaired(invoke)
    }

    @Command
    fun listPairedCallback(invoke: Invoke) = doListPaired(invoke)

    private fun doListPaired(invoke: Invoke) {
        val adapter = adapter()
        if (adapter == null) {
            invoke.reject("Bluetooth недоступен на этом устройстве")
            return
        }
        val devices = JSArray()
        adapter.bondedDevices?.forEach { d ->
            val obj = JSObject()
            obj.put("name", d.name ?: d.address)
            obj.put("address", d.address)
            devices.put(obj)
        }
        val ret = JSObject()
        ret.put("devices", devices)
        invoke.resolve(ret)
    }

    @Command
    fun connect(invoke: Invoke) {
        if (!ensureBluetoothPermission(invoke, "connectCallback")) return
        doConnect(invoke)
    }

    @Command
    fun connectCallback(invoke: Invoke) = doConnect(invoke)

    private fun doConnect(invoke: Invoke) {
        val address = invoke.getString("address")
        if (address.isNullOrBlank()) {
            invoke.reject("address обязателен")
            return
        }
        try {
            val adapter = adapter() ?: throw IOException("Bluetooth недоступен")
            adapter.cancelDiscovery()
            val device = adapter.getRemoteDevice(address)
            val s = device.createRfcommSocketToServiceRecord(sppUuid)
            s.connect() // блокирующий вызов; tauri-android выполняет команды в фоне
            socket = s
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Не удалось подключиться к $address: ${e.message}")
        }
    }

    @Command
    fun write(invoke: Invoke) {
        val b64 = invoke.getString("data")
        if (b64 == null) {
            invoke.reject("data обязательна")
            return
        }
        try {
            val bytes = Base64.decode(b64, Base64.NO_WRAP)
            val s = socket ?: throw IOException("нет соединения")
            s.outputStream.write(bytes)
            s.outputStream.flush()
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("write: ${e.message}")
        }
    }

    @Command
    fun read(invoke: Invoke) {
        try {
            val s = socket ?: throw IOException("нет соединения")
            val input = s.inputStream
            val available = input.available()
            val ret = JSObject()
            if (available <= 0) {
                ret.put("data", "")
            } else {
                val buf = ByteArray(available.coerceAtMost(4096))
                val n = input.read(buf)
                ret.put("data", Base64.encodeToString(buf, 0, n, Base64.NO_WRAP))
            }
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject("read: ${e.message}")
        }
    }

    @Command
    fun disconnect(invoke: Invoke) {
        try {
            socket?.close()
        } catch (_: Exception) {
        }
        socket = null
        invoke.resolve()
    }
}
