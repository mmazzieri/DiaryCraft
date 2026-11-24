package it.mauromazzieri.diarycraft

import android.content.ContentResolver
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import java.io.IOException

/**
 * The diary folder the person chose, reached through the system's document access: it may be on
 * the phone, on a memory card, or in a sync app that offers its folders to other apps.
 */
class Folder(private val resolver: ContentResolver, val tree: Uri) {
    data class Entry(val id: String, val name: String, val isDir: Boolean, val size: Long, val changed: Long)

    private val rootId: String = DocumentsContract.getTreeDocumentId(tree)

    private fun uri(id: String): Uri = DocumentsContract.buildDocumentUriUsingTree(tree, id)

    private fun children(parentId: String): List<Entry> {
        val list = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parentId)
        val columns = arrayOf(
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_SIZE,
            Document.COLUMN_LAST_MODIFIED,
        )
        val cursor = resolver.query(list, columns, null, null, null) ?: throw IOException("the folder could not be listed")
        return cursor.use { c ->
            val out = ArrayList<Entry>()
            while (c.moveToNext()) {
                val id = c.getString(0) ?: continue
                val name = c.getString(1) ?: continue
                val dir = c.getString(2) == Document.MIME_TYPE_DIR
                out.add(Entry(id, name, dir, if (c.isNull(3)) 0L else c.getLong(3), if (c.isNull(4)) 0L else c.getLong(4)))
            }
            out
        }
    }

    /** Every file two folders deep, by its path `YYYY/MM/name`. */
    fun files(): Map<String, Entry> {
        val out = HashMap<String, Entry>()
        for (y in children(rootId)) {
            if (!y.isDir || y.name.length != 4) continue
            for (m in children(y.id)) {
                if (!m.isDir || m.name.length != 2) continue
                for (f in children(m.id)) if (!f.isDir) out["${y.name}/${m.name}/${f.name}"] = f
            }
        }
        return out
    }

    /** The files of a month folder, by name; none when the folder is not there. */
    fun month(year: String, month: String): Map<String, Entry> {
        val y = children(rootId).firstOrNull { it.isDir && it.name == year } ?: return emptyMap()
        val m = children(y.id).firstOrNull { it.isDir && it.name == month } ?: return emptyMap()
        return children(m.id).filter { !it.isDir }.associateBy { it.name }
    }

    private fun dir(parentId: String, name: String): String {
        children(parentId).firstOrNull { it.isDir && it.name == name }?.let { return it.id }
        val made = DocumentsContract.createDocument(resolver, uri(parentId), Document.MIME_TYPE_DIR, name)
            ?: throw IOException("the folder $name could not be made")
        return DocumentsContract.getDocumentId(made)
    }

    fun read(id: String): ByteArray =
        resolver.openInputStream(uri(id))?.use { it.readBytes() } ?: throw IOException("the file could not be opened")

    /** Replaces the content of a file, or makes it, and reads it back. */
    fun write(path: String, bytes: ByteArray) {
        val (year, month, name) = path.split('/')
        val parent = dir(dir(rootId, year), month)
        val id = children(parent).firstOrNull { !it.isDir && it.name == name }?.id ?: create(parent, name)
        resolver.openOutputStream(uri(id), "wt")?.use { it.write(bytes) } ?: throw IOException("$name could not be written")
        if (!read(id).contentEquals(bytes)) throw IOException("$name was not written whole")
    }

    private fun create(parent: String, name: String): String {
        val made = DocumentsContract.createDocument(resolver, uri(parent), "application/octet-stream", name)
            ?: throw IOException("$name could not be made")
        val id = DocumentsContract.getDocumentId(made)
        // Another name means that a file with this one appeared meanwhile, or that the provider
        // changed the extension: the diary would not know it.
        if (children(parent).none { it.id == id && it.name == name }) {
            DocumentsContract.deleteDocument(resolver, uri(id))
            throw IOException("$name could not be made with this name")
        }
        return id
    }
}
