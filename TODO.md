# TODO
- `#·52`. Drag n Drop to create tiles
- `#·57`. Dynamic attributes
- `#·58`. Shortcut for style attributes
- `#·55`. Better way to do APIs. `server_fn`?
- `#·32`. Open source terrazzo client.
- `#·32`. Open source named.
- `#·32`. Open source autoclone.
- `#··8`. Resizeable component
- `#··6`. Text editor based on Monaco
- `#··7`. Integration with Language Server

# Nice to have
- `#·51`. Save terminal buffer state before close
- `#·39`. Reconnect tab after tab goes to sleep and stream times out.
- `#·45`. Lazily load tab
- `#·43`. Errors should implement a trait that indicates their http status code
- `#·33`. Simplify syntax for text nodes.
- `#·25`. Child nodes can be built out of any expression.

Next: `#·59`.

# DONE
- `#··3`. `idx: { idx+=1; idx }` does not work. Fixed with `key: "{tab.key()}"`
- `#··1`. Tracing with spans
- `#··2`. ReleaseOnDrop: Unable to release on drop. I haven't seen this one again in a while.
- `#··4`. Have a single stream open for all terminals or use h2
- `#··8`. Customize tab title -- made some optimizations
- `#··5`. Showing status parameters such as pwd and current branch -- polling for update? nah.
- `#··9`. Autoclone
- `#·10`. Find a simple way to create or update a DOM node
- `#·11`. Split framework from terrazzo app
- `#·13`. Create a proc macro that turns expressions into XElement
- `#·12`. Figure out element <-> signal recursive dep. If a reactive callback is dropped, it should be removed from the list of subscribers of the signal.
- `#·16`. Recompute index of nodes without key
- `#·20`. set_signal should only be created if the attribute is there
- `#·14`. Add support for events to the template engine
- `#·17`. Add support for dynamic XNodes
- `#·21`. Suport non-signal parameters passed to dynamic nodes
- `#·18`. Element nodes can be computed based on the previous value: REJECTED
- `#·22`. Use `mut` keywork for mutable signals, default is readonly
- `#·19`. Signals should have an `.update(f:Fn(old)->new)` function: NOT NEEDED
- `#·24`. Figure out why the build script needs to acquire the lock in retail mode.
- `#·26`. Unsubscribe from signals once the template return false
- `#·27`. Implement derived signal
- `#·28`. Dynamic templates should be applied once
- `#·29`. Tag names can also use macro syntax
- `#·30`. Autoclone fails if not used
- `#·31`. Implement tabs widget
- `#·23`. Implement APIs to list, create, delete, and stream a terminal.
- `#·34`. Tab descriptors don't need to implement Eq
- `#·37`. Separate tab titles vs tab ids
- `#·38`. tab ids must be unique and they aren't
- `#·40`. Tabs leak even though they are detached
- `#·36`. Fix CSS
- `#·41`. Fix resize
- `#·44`. Fix close
- `#·42`. Call the list API when first loading the tabs
- `#·43`. Single stream for all tabs
- `#·15`. Add a prelude with all the useful imports used by codegen
- `#·48`. Better job at packaging the app. Copy the assets to the target folder
- `#·50`. Edit terminal names
- `#·49`. Do a better job at handling side effects in nodes
- `#·47`. Allocate tab IDs from the backend.
- `#·53`. Cleanup debug traces before open-sourcing terrazzo client
- `#·54`. GitHub actions
- `#·35`. Fix bug in rendering when element keys are not defined
- `#·46`. What happens if processes run in the background with no tab attached?: Process hangs as it can't continue writing to stdout.
- `#·56`. Improve documentation
