package com.recuscan.btspp

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.DocumentsContract
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.util.Base64
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.io.IOException
import java.net.URLDecoder
import java.util.UUID

/**
 * Bluetooth Classic SPP (RFCOMM) канал до ELM327-адаптера.
 * Адаптер должен быть предварительно сопряжён в настройках Android.
 *
 * API tauri-android сверено с 2.12.5: аргументы через invoke.getArgs(),
 * колбэки разрешений — @PermissionCallback.
 * TODO(on-device): прогнать запрос разрешений на реальном устройстве.
 */
@SuppressLint("MissingPermission") // разрешения проверяются в ensureBluetoothPermission
@TauriPlugin(
    permissions = [
        Permission(
            strings = [
                Manifest.permission.BLUETOOTH_CONNECT,
                Manifest.permission.BLUETOOTH_SCAN
            ],
            alias = "bluetooth"
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

    /**
     * Запрашивает BLUETOOTH_CONNECT и BLUETOOTH_SCAN на API 31+, иначе возвращает true сразу.
     * SCAN нужен: createRfcommSocketToServiceRecord()/connect() внутри вызывают cancelDiscovery(),
     * которое на Android 12+ требует именно BLUETOOTH_SCAN.
     */
    private fun ensureBluetoothPermission(invoke: Invoke, callback: String): Boolean {
        val granted = Build.VERSION.SDK_INT < 31 || (
            activity.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) ==
                PackageManager.PERMISSION_GRANTED &&
            activity.checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) ==
                PackageManager.PERMISSION_GRANTED
        )
        if (!granted) {
            requestPermissionForAlias("bluetooth", invoke, callback)
        }
        return granted
    }

    @Command
    fun listPaired(invoke: Invoke) {
        if (!ensureBluetoothPermission(invoke, "listPairedCallback")) return
        doListPaired(invoke)
    }

    @PermissionCallback
    private fun listPairedCallback(invoke: Invoke) = doListPaired(invoke)

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

    @PermissionCallback
    private fun connectCallback(invoke: Invoke) = doConnect(invoke)

    private fun doConnect(invoke: Invoke) {
        val address = invoke.getArgs().getString("address", null)
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
        val b64 = invoke.getArgs().getString("data", null)
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

    // ---------- Хранилище экспорта (SAF + MediaStore) ----------
    //
    // Публичные папки (Загрузки/Документы) на Android 10+ недоступны по «сырым»
    // путям из-за scoped storage, а приватная Android/data не видна пользователю
    // в файловых менеджерах. Поэтому:
    //  - выбор папки — через SAF (ACTION_OPEN_DOCUMENT_TREE), URI сохраняем в Rust;
    //  - папка по умолчанию — MediaStore Downloads/recuscan (видна в «Файлах»);
    //  - на API < 29 (MediaStore.Downloads недоступен) — app-specific папка,
    //    которую на тех версиях файловые менеджеры ещё открывают.

    private fun optString(args: JSObject, key: String): String? {
        if (!args.has(key) || args.isNull(key)) return null
        return args.getString(key, null)
    }

    /** Человекочитаемое имя SAF-дерева: "primary:Documents/recuscan" → "Documents/recuscan". */
    private fun treeDisplayName(treeUri: Uri): String {
        return try {
            val id = DocumentsContract.getTreeDocumentId(treeUri)
            val decoded = URLDecoder.decode(id, "UTF-8")
            when {
                decoded.startsWith("primary:") -> decoded.removePrefix("primary:").ifBlank { "Внутренняя память" }
                decoded.startsWith("home:") -> decoded.removePrefix("home:")
                else -> decoded
            }
        } catch (_: Exception) {
            "Выбранная папка"
        }
    }

    @Command
    fun pickFolder(invoke: Invoke) {
        try {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
            startActivityForResult(invoke, intent, "pickFolderResult")
        } catch (e: Exception) {
            invoke.reject("Не удалось открыть выбор папки: ${e.message}")
        }
    }

    @ActivityCallback
    private fun pickFolderResult(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.resolve(JSObject()) // отмена — пустой ответ
            return
        }
        val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        try {
            activity.contentResolver.takePersistableUriPermission(uri, flags)
        } catch (_: Exception) {
            // Некоторые провайдеры не дают persistable-прав: URI живёт до перезагрузки
        }
        val ret = JSObject()
        ret.put("uri", uri.toString())
        ret.put("name", treeDisplayName(uri))
        invoke.resolve(ret)
    }

    @Command
    fun pickLogFile(invoke: Invoke) {
        try {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                putExtra(
                    Intent.EXTRA_MIME_TYPES,
                    arrayOf(
                        "text/*",
                        "application/json",
                        "text/csv",
                        "text/comma-separated-values",
                        "application/vnd.ms-excel"
                    )
                )
            }
            startActivityForResult(invoke, intent, "pickLogFileResult")
        } catch (e: Exception) {
            invoke.reject("Не удалось открыть выбор файла: ${e.message}")
        }
    }

    @ActivityCallback
    private fun pickLogFileResult(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.resolve(JSObject())
            return
        }
        try {
            activity.contentResolver.takePersistableUriPermission(
                uri, Intent.FLAG_GRANT_READ_URI_PERMISSION
            )
        } catch (_: Exception) {
        }
        val ret = JSObject()
        ret.put("uri", uri.toString())
        ret.put("name", queryDisplayName(uri) ?: uri.lastPathSegment ?: "лог")
        invoke.resolve(ret)
    }

    private fun queryDisplayName(uri: Uri): String? {
        return try {
            activity.contentResolver.query(uri, null, null, null, null)?.use { c ->
                val i = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (i >= 0 && c.moveToFirst()) c.getString(i) else null
            }
        } catch (_: Exception) {
            null
        }
    }

    /** Найти дочерний документ дерева по имени (для перезаписи без дублей). */
    private fun findChild(treeUri: Uri, parentDocId: String, displayName: String): Uri? {
        val children = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
        return try {
            activity.contentResolver.query(
                children,
                arrayOf(
                    DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                    DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                    DocumentsContract.Document.COLUMN_MIME_TYPE
                ), null, null, null
            )?.use { c ->
                while (c.moveToNext()) {
                    val mime = c.getString(2)
                    if (mime == DocumentsContract.Document.MIME_TYPE_DIR) continue
                    if (c.getString(1) == displayName) {
                        return DocumentsContract.buildDocumentUriUsingTree(treeUri, c.getString(0))
                    }
                }
                null
            }
        } catch (_: Exception) {
            null
        }
    }

    @Command
    fun saveExport(invoke: Invoke) {
        val args = invoke.getArgs()
        val fileName = optString(args, "fileName")
        val dataB64 = optString(args, "data")
        if (fileName.isNullOrBlank() || dataB64 == null) {
            invoke.reject("fileName и data обязательны")
            return
        }
        val mime = optString(args, "mime") ?: "application/octet-stream"
        val treeUriStr = optString(args, "treeUri")
        try {
            val data = Base64.decode(dataB64, Base64.NO_WRAP)
            val (uri, label) = if (!treeUriStr.isNullOrBlank()) {
                writeToTree(Uri.parse(treeUriStr), fileName, mime, data)
            } else {
                writeToDefault(fileName, mime, data)
            }
            val ret = JSObject()
            ret.put("uri", uri)
            ret.put("label", label)
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject("Не удалось сохранить $fileName: ${e.message}")
        }
    }

    private fun writeToTree(treeUri: Uri, fileName: String, mime: String, data: ByteArray): Pair<String, String> {
        val resolver = activity.contentResolver
        val docId = DocumentsContract.getTreeDocumentId(treeUri)
        val parentDoc = DocumentsContract.buildDocumentUriUsingTree(treeUri, docId)
        // Перезаписываем: иначе провайдер создаст "name (1).csv" при совпадении имён
        findChild(treeUri, docId, fileName)?.let {
            try { DocumentsContract.deleteDocument(resolver, it) } catch (_: Exception) {}
        }
        val docUri = DocumentsContract.createDocument(resolver, parentDoc, mime, fileName)
            ?: throw IOException("провайдер не создал документ")
        resolver.openOutputStream(docUri, "wt")?.use { it.write(data) }
            ?: throw IOException("нет потока записи")
        return Pair(docUri.toString(), "${treeDisplayName(treeUri)}/$fileName")
    }

    private fun writeToDefault(fileName: String, mime: String, data: ByteArray): Pair<String, String> {
        return if (Build.VERSION.SDK_INT >= 29) {
            // Публичные Загрузки без разрешений — для своих файлов (API 29+)
            val values = ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, fileName)
                put(MediaStore.Downloads.MIME_TYPE, mime)
                put(MediaStore.Downloads.RELATIVE_PATH, "Download/recuscan/")
                put(MediaStore.Downloads.IS_PENDING, 1)
            }
            val resolver = activity.contentResolver
            val uri = resolver.insert(
                MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY), values
            ) ?: throw IOException("MediaStore не создал запись")
            try {
                resolver.openOutputStream(uri)?.use { it.write(data) }
                    ?: throw IOException("нет потока записи")
            } catch (e: Exception) {
                resolver.delete(uri, null, null)
                throw e
            }
            values.clear()
            values.put(MediaStore.Downloads.IS_PENDING, 0)
            resolver.update(uri, values, null, null)
            Pair(uri.toString(), "Загрузки/recuscan/$fileName")
        } else {
            // API 24–28: публичные Загрузки требуют WRITE_EXTERNAL_STORAGE.
            // Пишем в app-specific папку: на этих версиях файловые менеджеры её открывают.
            val base = activity.getExternalFilesDir(Environment.DIRECTORY_DOCUMENTS)
                ?: throw IOException("внешнее хранилище недоступно")
            val dir = File(base, "recuscan")
            if (!dir.exists() && !dir.mkdirs()) throw IOException("не создать ${dir.path}")
            val file = File(dir, fileName)
            file.writeBytes(data)
            Pair(Uri.fromFile(file).toString(), file.path)
        }
    }

    @Command
    fun listExports(invoke: Invoke) {
        val treeUriStr = optString(invoke.getArgs(), "treeUri")
        try {
            val files = JSArray()
            if (!treeUriStr.isNullOrBlank()) {
                listTree(Uri.parse(treeUriStr), files)
            } else {
                listDefault(files)
            }
            val ret = JSObject()
            ret.put("files", files)
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject("Не удалось прочитать список логов: ${e.message}")
        }
    }

    private fun putFile(files: JSArray, uri: String, name: String, size: Long, mtime: Long) {
        val obj = JSObject()
        obj.put("uri", uri)
        obj.put("name", name)
        obj.put("size", size)
        obj.put("mtime", mtime)
        files.put(obj)
    }

    private fun listTree(treeUri: Uri, files: JSArray) {
        val docId = DocumentsContract.getTreeDocumentId(treeUri)
        val children = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, docId)
        activity.contentResolver.query(
            children,
            arrayOf(
                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                DocumentsContract.Document.COLUMN_MIME_TYPE,
                DocumentsContract.Document.COLUMN_SIZE,
                DocumentsContract.Document.COLUMN_LAST_MODIFIED
            ), null, null, null
        )?.use { c ->
            while (c.moveToNext()) {
                if (c.getString(2) == DocumentsContract.Document.MIME_TYPE_DIR) continue
                val docUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, c.getString(0))
                putFile(files, docUri.toString(), c.getString(1) ?: "лог", c.getLong(3), c.getLong(4))
            }
        }
    }

    private fun listDefault(files: JSArray) {
        if (Build.VERSION.SDK_INT >= 29) {
            val collection = MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
            activity.contentResolver.query(
                collection,
                arrayOf(
                    MediaStore.Downloads._ID,
                    MediaStore.Downloads.DISPLAY_NAME,
                    MediaStore.Downloads.SIZE,
                    MediaStore.Downloads.DATE_MODIFIED
                ),
                "${MediaStore.Downloads.RELATIVE_PATH} LIKE 'Download/recuscan/%'",
                null, null
            )?.use { c ->
                while (c.moveToNext()) {
                    val uri = ContentUris.withAppendedId(collection, c.getLong(0))
                    putFile(files, uri.toString(), c.getString(1) ?: "лог", c.getLong(2), c.getLong(3) * 1000)
                }
            }
        } else {
            val base = activity.getExternalFilesDir(Environment.DIRECTORY_DOCUMENTS) ?: return
            File(base, "recuscan").listFiles()?.forEach { f ->
                if (f.isFile) putFile(files, Uri.fromFile(f).toString(), f.name, f.length(), f.lastModified())
            }
        }
    }

    /** Прочитать файл по content:// или file:// URI. Лимит 64 МБ — защита от OOM в WebView. */
    @Command
    fun readExport(invoke: Invoke) {
        val uriStr = optString(invoke.getArgs(), "uri")
        if (uriStr.isNullOrBlank()) {
            invoke.reject("uri обязателен")
            return
        }
        try {
            val bytes = if (uriStr.startsWith("content://")) {
                activity.contentResolver.openInputStream(Uri.parse(uriStr))?.use { input ->
                    val out = java.io.ByteArrayOutputStream()
                    val buf = ByteArray(64 * 1024)
                    var total = 0
                    while (true) {
                        val n = input.read(buf)
                        if (n < 0) break
                        total += n
                        if (total > 64 * 1024 * 1024) throw IOException("файл больше 64 МБ")
                        out.write(buf, 0, n)
                    }
                    out.toByteArray()
                } ?: throw IOException("не открыть файл")
            } else {
                val file = File(Uri.parse(uriStr).path ?: uriStr)
                if (file.length() > 64 * 1024 * 1024) throw IOException("файл больше 64 МБ")
                file.readBytes()
            }
            val ret = JSObject()
            ret.put("data", Base64.encodeToString(bytes, Base64.NO_WRAP))
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject("Не удалось прочитать файл: ${e.message}")
        }
    }
}
