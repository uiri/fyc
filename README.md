# Forget Yo Container

`fyc` is an implementation of an App Container Executor per the [App Container Spec](https://github.com/appc/spec#app-container).

This project has two goals:

 - Do the least amout of isolation required to run an ACI (defined per the spec)
 - Pass the ACE validator whose code is bundled with the spec

Right now, this means chrooting the application, setting some environment variables and providing a metadata service.

> An executor MAY igore isolators that it does not understand and run the pod without them.

`fyc` remains a spec-compliant ACE despite not understanding any isolators. If the spec attempts to verify some well known isolators, `fyc` will fail these checks. These isolators might be implemented, eventually.
