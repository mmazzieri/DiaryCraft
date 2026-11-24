package it.mauromazzieri.diarycraft

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/** A day as it was read: what a save compares the folder with. */
data class Loaded(val date: String, val path: String?, val base: String?, val encoding: String?, val text: String)

data class Hit(val date: String, val snippet: String)

/** How a save went. [loaded] is what the editor goes on from. */
data class Saved(val loaded: Loaded, val copy: String?)

data class DiaryState(
    val ready: Boolean = false,
    val hasFolder: Boolean = false,
    /** The folder chosen. */
    val folder: String = "",
    val busy: Boolean = false,
    /** Days with an entry, newest first. */
    val days: List<String> = emptyList(),
    val conflicts: List<String> = emptyList(),
    val error: String? = null,
    /** A conflict copy that a save wrote. */
    val notice: String? = null,
)

/**
 * The diary folder. All file work runs on the IO dispatcher, one task at a time. Before a text is
 * written it is kept in the app's own storage, and leaves it only when the folder holds it: the
 * document access has no atomic rename, so a write that stops halfway must lose nothing.
 */
object Diary {
    private const val PREFS = "diarycraft"
    private val lock = Mutex()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val mutableState = MutableStateFlow(DiaryState())
    val state: StateFlow<DiaryState> = mutableState

    private lateinit var app: Context
    private var folder: Folder? = null
    private var files: Map<String, Folder.Entry> = emptyMap()
    private var paths: Map<String, String> = emptyMap()
    /** Texts read for the list and the search, by file id, size and time. */
    private val texts = HashMap<Triple<String, Long, Long>, String>()

    fun start(context: Context) {
        if (::app.isInitialized) return
        app = context.applicationContext
        scope.launch {
            lock.withLock {
                val saved = prefs().getString("folder", null)?.let(Uri::parse)
                val allowed = saved != null && app.contentResolver.persistedUriPermissions.any {
                    it.uri == saved && it.isReadPermission && it.isWritePermission
                }
                if (allowed && saved != null) {
                    folder = Folder(app.contentResolver, saved)
                    load()
                }
                mutableState.value = mutableState.value.copy(ready = true, hasFolder = folder != null, folder = folder?.tree.toString())
            }
        }
    }

    fun choose(tree: Uri) {
        scope.launch {
            lock.withLock {
                val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                runCatching { app.contentResolver.takePersistableUriPermission(tree, flags) }
                prefs().edit().putString("folder", tree.toString()).apply()
                folder = Folder(app.contentResolver, tree)
                texts.clear()
                mutableState.value = DiaryState(ready = true, hasFolder = true, folder = tree.toString())
                load()
            }
        }
    }

    /** Looks at the folder again: other computers may have changed it. */
    fun reload() {
        scope.launch { lock.withLock { if (folder != null) load() } }
    }

    fun dismiss() {
        mutableState.value = mutableState.value.copy(error = null, notice = null)
    }

    private fun load() {
        mutableState.value = mutableState.value.copy(busy = true)
        runCatching {
            recover()
            scan()
        }.onFailure(::fail)
        mutableState.value = mutableState.value.copy(busy = false)
    }

    private fun scan() {
        val f = folder ?: return
        files = f.files()
        val answer = Core.ask("scan") { put("files", JSONArray(files.keys.toList())) } as JSONObject
        val entries = rows(answer.getJSONArray("entries"))
        paths = entries.toMap()
        mutableState.value = mutableState.value.copy(
            days = entries.map { it.first },
            conflicts = rows(answer.getJSONArray("conflicts")).map { it.second },
        )
    }

    private fun rows(a: JSONArray): List<Pair<String, String>> =
        (0 until a.length()).map { a.getJSONObject(it).let { o -> o.getString("date") to o.getString("path") } }

    private fun raw(bytes: ByteArray) = String(bytes, Charsets.ISO_8859_1)

    private fun decode(bytes: ByteArray) = Core.ask("decode") { put("raw", raw(bytes)) } as JSONObject

    /** Reads a day, or null when it cannot be read: it must not look like an empty one. */
    suspend fun read(date: String): Loaded? = withContext(Dispatchers.IO) {
        lock.withLock { runCatching { readNow(date) }.onFailure(::fail).getOrNull() }
    }

    private fun readNow(date: String): Loaded {
        val f = folder ?: throw CoreException("no folder is open")
        val path = paths[date] ?: return Loaded(date, null, null, null, "")
        val (year, month, name) = path.split('/')
        val entry = f.month(year, month)[name] ?: return Loaded(date, null, null, null, "")
        val d = decode(f.read(entry.id))
        return Loaded(date, path, d.getString("version"), d.getString("encoding"), d.getString("text"))
    }

    /** Saves a text. Null when it could not be written: the draft stays, and is tried again. */
    suspend fun save(loaded: Loaded, text: String): Saved? = withContext(Dispatchers.IO) {
        lock.withLock {
            runCatching {
                val draft = keep(loaded, text)
                val saved = write(loaded, text)
                draft.delete()
                saved
            }.onFailure(::fail).getOrNull()
        }
    }

    private fun write(loaded: Loaded, text: String): Saved {
        val f = folder ?: throw CoreException("no folder is open")
        val (year, month) = loaded.date.split('-')
        val names = f.month(year, month)
        val path = loaded.path ?: "$year/$month/${loaded.date}.md"
        val disk = names[path.substringAfterLast('/')]?.let { f.read(it.id) }
        val plan = Core.ask("plan") {
            put("date", loaded.date)
            put("path", path)
            put("base", loaded.base ?: JSONObject.NULL)
            put("encoding", loaded.encoding ?: JSONObject.NULL)
            put("disk", disk?.let(::raw) ?: JSONObject.NULL)
            put("text", text)
            put("label", deviceName())
            put("stamp", SimpleDateFormat("yyyyMMdd-HHmmss", Locale.ROOT).format(Date()))
            put("names", JSONArray(names.keys.toList()))
        } as JSONObject
        return when (plan.getString("do")) {
            "write" -> {
                f.write(path, plan.getString("raw").toByteArray(Charsets.ISO_8859_1))
                if (loaded.base == null) scan()
                Saved(Loaded(loaded.date, path, plan.getString("version"), loaded.encoding, text), null)
            }
            "conflict" -> {
                val copy = plan.getString("path")
                f.write(copy, plan.getString("raw").toByteArray(Charsets.ISO_8859_1))
                scan()
                mutableState.value = mutableState.value.copy(notice = copy)
                Saved(readNow(loaded.date), copy)
            }
            // Blank and new, or written already by a save that did not end.
            else -> Saved(if (disk != null) readNow(loaded.date) else loaded, null)
        }
    }

    /** The text in the app's own storage until the folder holds it. */
    private fun keep(loaded: Loaded, text: String): File {
        val name = "${loaded.date}-${folder?.tree.toString().hashCode()}"
        val file = File(drafts(), "$name.json")
        val part = File(drafts(), "$name.part")
        val o = JSONObject().put("folder", folder?.tree.toString()).put("date", loaded.date).put("text", text)
        loaded.path?.let { o.put("path", it) }
        loaded.base?.let { o.put("base", it) }
        loaded.encoding?.let { o.put("encoding", it) }
        part.outputStream().use {
            it.write(o.toString().toByteArray())
            it.fd.sync()
        }
        if (!part.renameTo(file)) throw CoreException("the draft could not be kept")
        return file
    }

    /** Writes the texts that an earlier save could not. */
    private fun recover() {
        for (file in drafts().listFiles { f -> f.name.endsWith(".json") } ?: emptyArray()) {
            // One that cannot be written now does not stop the others.
            runCatching {
                val o = JSONObject(file.readText())
                if (o.getString("folder") != folder?.tree.toString()) return@runCatching
                fun field(k: String) = if (o.has(k)) o.getString(k) else null
                val text = o.getString("text")
                write(Loaded(o.getString("date"), field("path"), field("base"), field("encoding"), text), text)
                file.delete()
            }.onFailure(::fail)
        }
    }

    private fun text(date: String): String? {
        val f = folder ?: return null
        val entry = paths[date]?.let { files[it] } ?: return null
        val key = Triple(entry.id, entry.size, entry.changed)
        texts[key]?.let { return it }
        return decode(f.read(entry.id)).getString("text").also { texts[key] = it }
    }

    suspend fun title(date: String): String = withContext(Dispatchers.IO) {
        lock.withLock {
            runCatching { (Core.ask("summary") { put("text", text(date) ?: "") } as JSONObject).getString("title") }.getOrDefault("")
        }
    }

    suspend fun search(query: String): List<Hit> = withContext(Dispatchers.IO) {
        val list = JSONArray()
        // The lock is taken for each day, so that a save does not wait for the whole diary.
        for (date in mutableState.value.days) {
            lock.withLock {
                runCatching { text(date) }.getOrNull()?.let { list.put(JSONObject().put("date", date).put("text", it)) }
            }
        }
        runCatching {
            val hits = Core.ask("search") { put("entries", list); put("query", query) } as JSONArray
            (0 until hits.length()).map { hits.getJSONObject(it).let { h -> Hit(h.getString("date"), h.getString("snippet")) } }
        }.onFailure(::fail).getOrDefault(emptyList())
    }

    private fun fail(e: Throwable) {
        if (e is kotlinx.coroutines.CancellationException) throw e
        mutableState.value = mutableState.value.copy(error = e.message ?: e.toString())
    }

    private fun prefs() = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun drafts(): File = File(app.filesDir, "drafts").apply { mkdirs() }

    private fun deviceName(): String =
        Settings.Global.getString(app.contentResolver, Settings.Global.DEVICE_NAME) ?: Build.MODEL
}
