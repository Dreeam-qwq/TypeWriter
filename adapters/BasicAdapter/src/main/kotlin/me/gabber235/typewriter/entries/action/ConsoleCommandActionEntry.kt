package me.gabber235.typewriter.entries.action

import com.github.shynixn.mccoroutine.bukkit.launch
import lirand.api.extensions.server.commands.dispatchCommand
import me.gabber235.typewriter.adapters.Colors
import me.gabber235.typewriter.adapters.Entry
import me.gabber235.typewriter.adapters.modifiers.Help
import me.gabber235.typewriter.adapters.modifiers.Placeholder
import me.gabber235.typewriter.entry.Criteria
import me.gabber235.typewriter.entry.Modifier
import me.gabber235.typewriter.entry.entries.ActionEntry
import me.gabber235.typewriter.extensions.placeholderapi.parsePlaceholders
import me.gabber235.typewriter.plugin
import me.gabber235.typewriter.utils.Icons
import org.bukkit.Bukkit
import org.bukkit.entity.Player

@Entry("console_run_command", "Run command from console", Colors.RED, Icons.TERMINAL)
/**
 * The Console Command Action is an action that sends a command to the server console. This action provides you with the ability to execute console commands on the server in response to specific events.
 *
 * ## How could this be used?
 *
 * This action can be useful in a variety of situations. You can use it to perform administrative tasks, such as sending a message to all players on the server, or to automate server tasks, such as setting the time of day or weather conditions. The possibilities are endless!
 */
class ConsoleCommandActionEntry(
    override val id: String = "",
    override val name: String = "",
    override val criteria: List<Criteria> = emptyList(),
    override val modifiers: List<Modifier> = emptyList(),
    override val triggers: List<String> = emptyList(),
    @Placeholder
    @Help("The command to run. (Use %player_name% for the player's name)")
    // The command that the console will run.
    private val command: String = "",
) : ActionEntry {
    override fun execute(player: Player) {
        super.execute(player)
        // Run in the main thread
        plugin.launch {
            Bukkit.getConsoleSender().dispatchCommand(command.parsePlaceholders(player))
        }
    }
}