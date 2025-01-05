### padmixer

#### todo

- at the moment i am trying to communicate between my controller input caching thread and my ui components. thus far it has been an annoying process. to use channels, i have to be in an async context. it seems okay to update my simplified pad input cache in one thread, and proceed to produce desktop-wide virtual keyboard input. however, connecting that data to UI values remains a puzzle to me. I have been enjoying the Rust GTK4 book, but for some reason I think I am lacking some important understanding of UI authoring using these APIs, because it is not obvious nor intuitive to me how to send them data unless it is explicitly stated. but then i realized the difficulty was in finding a callback that triggered not on UI interaction, but on my own message passing. it occurred to me that if i have a callback that can be attached to a signal, then perhaps i can emit the signal in the input processing thread. but that seems far too complicated to be necessary, so I am continuing to take my time puzzling this out.

