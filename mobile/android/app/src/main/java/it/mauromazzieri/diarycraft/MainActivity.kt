package it.mauromazzieri.diarycraft

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private val Blue = Color(0xFF2F4B7C)
private val PaleBlue = Color(0xFFB7C4DC)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        Diary.start(this)
        setContent {
            val colors = if (isSystemInDarkTheme()) darkColorScheme(primary = PaleBlue) else lightColorScheme(primary = Blue)
            MaterialTheme(colorScheme = colors) { Surface(Modifier.fillMaxSize()) { App() } }
        }
    }

    override fun onResume() {
        super.onResume()
        Diary.reload()
    }
}

private fun show(date: String): String = LocalDate.parse(date).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.FULL))

@Composable
private fun App() {
    val state by Diary.state.collectAsStateWithLifecycle()
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { if (it != null) Diary.choose(it) }
    var day by rememberSaveable { mutableStateOf<String?>(null) }
    if (!state.ready) return
    val open = day
    when {
        !state.hasFolder -> Welcome { pick.launch(null) }
        open == null -> Days(state, pick = { pick.launch(null) }, open = { day = it })
        else -> Editor(open, state) { day = null }
    }
}

@Composable
private fun Welcome(choose: () -> Unit) {
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(stringResource(R.string.app_name), style = MaterialTheme.typography.headlineMedium)
        Text(stringResource(R.string.welcome), modifier = Modifier.padding(vertical = 16.dp))
        Button(onClick = choose) { Text(stringResource(R.string.choose_folder)) }
    }
}

@Composable
private fun Messages(state: DiaryState) {
    val message = state.error ?: state.notice?.let { stringResource(R.string.conflict_saved, it) } ?: return
    Row(Modifier.fillMaxWidth().padding(start = 16.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(message, Modifier.weight(1f), color = MaterialTheme.colorScheme.error)
        TextButton(onClick = Diary::dismiss) { Text(stringResource(R.string.close)) }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Days(state: DiaryState, pick: () -> Unit, open: (String) -> Unit) {
    var query by rememberSaveable { mutableStateOf("") }
    var hits by remember { mutableStateOf<List<Hit>?>(null) }
    var picking by remember { mutableStateOf(false) }
    LaunchedEffect(query, state.days) {
        if (query.isBlank()) {
            hits = null
        } else {
            delay(400)
            hits = Diary.search(query)
        }
    }
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.app_name)) },
                actions = {
                    IconButton(onClick = { picking = true }) { Icon(Icons.Filled.DateRange, stringResource(R.string.other_day)) }
                    IconButton(onClick = Diary::reload) { Icon(Icons.Filled.Refresh, stringResource(R.string.reload)) }
                    TextButton(onClick = pick) { Text(stringResource(R.string.change_folder)) }
                },
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { open(LocalDate.now().toString()) },
                icon = { Icon(Icons.Filled.Edit, stringResource(R.string.today)) },
                text = { Text(stringResource(R.string.today)) },
            )
        },
    ) { inner ->
        Column(Modifier.padding(inner).fillMaxSize()) {
            if (state.busy) LinearProgressIndicator(Modifier.fillMaxWidth())
            Messages(state)
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text(stringResource(R.string.search)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
            )
            if (state.conflicts.isNotEmpty()) {
                Text(
                    stringResource(R.string.conflicts, state.conflicts.size),
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
                )
            }
            val found = hits
            LazyColumn {
                if (found != null) {
                    items(found) { DayRow(it.date, it.snippet) { open(it.date) } }
                } else {
                    items(state.days, key = { it }) { date ->
                        var title by remember(date) { mutableStateOf("") }
                        LaunchedEffect(date, state.busy) { title = Diary.title(date) }
                        DayRow(date, title) { open(date) }
                    }
                }
            }
            if (found?.isEmpty() == true || (found == null && state.days.isEmpty() && !state.busy)) {
                Text(stringResource(R.string.empty), Modifier.padding(24.dp))
            }
        }
    }
    if (picking) {
        val picker = rememberDatePickerState()
        DatePickerDialog(
            onDismissRequest = { picking = false },
            confirmButton = {
                TextButton(onClick = {
                    picking = false
                    picker.selectedDateMillis?.let { open(Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().toString()) }
                }) { Text(stringResource(R.string.open)) }
            },
            dismissButton = { TextButton(onClick = { picking = false }) { Text(stringResource(R.string.close)) } },
        ) { DatePicker(picker) }
    }
}

@Composable
private fun DayRow(date: String, text: String, open: () -> Unit) {
    Column(Modifier.fillMaxWidth().clickable(onClick = open).padding(horizontal = 16.dp, vertical = 10.dp)) {
        Text(show(date), maxLines = 1, overflow = TextOverflow.Ellipsis)
        if (text.isNotEmpty()) {
            Text(text, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
    HorizontalDivider()
}

/** The pages opened, so that coming back to one finds the text it had, saved or not. */
private val pages = HashMap<String, Page>()

/** What the editor of a day holds; it outlives the screen, so that leaving it still saves. */
private class Page(val date: String) {
    val text = mutableStateOf<String?>(null)
    private var loaded: Loaded? = null
    private val saving = Mutex()

    suspend fun read() {
        val now = loaded
        // Reloaded only when nothing is waiting to be saved.
        if (now != null && text.value != now.text) return
        val fresh = Diary.read(date) ?: return
        if (loaded === now && text.value == now?.text) {
            loaded = fresh
            text.value = fresh.text
        }
    }

    fun save() = Diary.scope.launch {
        saving.withLock {
            val base = loaded ?: return@withLock
            val written = text.value ?: return@withLock
            if (written == base.text) return@withLock
            val saved = Diary.save(base, written) ?: return@withLock
            loaded = saved.loaded
            // After a conflict the editor goes on from the folder's text, unless more was written.
            if (text.value == written) text.value = saved.loaded.text
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Editor(date: String, state: DiaryState, close: () -> Unit) {
    val page = remember(date, state.folder) { pages.getOrPut("${state.folder} $date") { Page(date) } }
    val text by page.text
    LaunchedEffect(page, state.days) { page.read() }
    LaunchedEffect(page, text) {
        delay(1000)
        page.save()
    }
    LifecycleEventEffect(Lifecycle.Event.ON_PAUSE) { page.save() }
    val leave = {
        page.save()
        close()
    }
    BackHandler(onBack = leave)
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(show(date), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = leave) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) }
                },
            )
        },
    ) { inner ->
        Column(Modifier.padding(inner).fillMaxSize().imePadding()) {
            Messages(state)
            TextField(
                value = text ?: "",
                onValueChange = { page.text.value = it },
                enabled = text != null,
                placeholder = { Text(stringResource(R.string.write_here)) },
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}
