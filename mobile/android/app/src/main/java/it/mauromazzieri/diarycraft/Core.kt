package it.mauromazzieri.diarycraft

import org.json.JSONObject

/** An error answered by DiaryCraft's engine. */
class CoreException(message: String) : Exception(message)

/**
 * DiaryCraft's engine, the same code as on the desktop (crates/ffi): one call, JSON in and
 * JSON out. The requests are listed in docs/mobile.md.
 */
object Core {
    init {
        System.loadLibrary("diarycraft_ffi")
    }

    @JvmStatic
    external fun call(request: String): String

    /** Runs a request and gives back what it answers, or throws [CoreException]. */
    fun ask(op: String, fill: JSONObject.() -> Unit = {}): Any? {
        val request = JSONObject().put("op", op).apply(fill)
        val answer = JSONObject(call(request.toString()))
        if (answer.has("error")) throw CoreException(answer.optString("error"))
        return answer.opt("ok")
    }
}
