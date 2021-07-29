#  IN SPACE IN SPACESHIP DEVDOCS

## TODO

- [ ] Surmount technical hurdles
- [ ] Refine loop

## design doc

### What we want

#### Prior art

- Mount & Blade
- Void Destroters

#### Focus areas

- Personal combat
- RTS elements
- Living breating world
- Sandbox

### AI

If we agree that AI needs three abilities:

- Sensors
- Decision making
- Actuation

...we need to design each in an ECS manner.

#### Sensors

##### Idea 0

Some components that hold state and systems that update them every frame. The decision systems can then look at the components or something.

#### Decision Making

##### Idea 0

Utility AI.

#### Actuation

Basic BehaviorTrees that are used for micro deciesion making for sequencing actions. As outlined by [Bobby Anguelov](https://takinginitiative.files.wordpress.com/2020/01/behaviortrees_breaking-the-cycle-of-misuse.pdf), small trees who only affect decisions related to the specfic task, we can avoid complex, hard to extend trees everyone complains about. How will this work out in practice, we'll see I suppose.

##### CraftEngine input

For this, I think I'll go back to SteeringRoutines from ISIS Godot.Basic units of logic that recieve some input and output the acceleration or velocity desired the next frame.

##### CraftArms input

TODO.

## devlog
