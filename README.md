# Forget Yo Container

`fyc` is an implementation of an App Container Executor per the [App Container Spec](https://github.com/appc/spec#app-container).

This project has two goals:

 - Do the least amout of isolation required to run an ACI (defined per the spec)
 - Pass the ACE validator whose code is bundled with the spec

Right now, this means chrooting the application, setting some environment variables and providing a metadata service.
