#  IN SPACE IN SPACESHIP DEVDOCS

WORLD TO LOCAL - inverse transformation you idiot!

## TODO

- [ ] Surmount technical hurdles
- [ ] Refine loop

- crates to check out
 - [ ] bevy polyline
 - [ ] big brain
 - [ ] bevy remote dev tools

## design doc

### What we want

#### Prior art

- Mount & Blade
- Void Destrotters

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

- Layers
    - Sensors
        - Where do they store their stimuli?
            - Must be general enough to allow multiple descision makers to read from them.
            - Blackboard
                - Entities?
                - Resources?
                - Hybrid?
        - Do they run every frame?
        - Idea 0
            - Entity outputs
            - Run every frame.
    - Decision making
        - State
            - Blackboard
        - A need for frame distribution?
        - Where do they store their results?
            - Specialized?
        - Idea 0
            - Behavior Trees
        - Idea 1
            - Utility AI
                - Big Brain
    - Actuation
        - Specialized per task?
        - Multi frame actions?
            - Action cancelling
                - Stateful?
        - CratEngine input
            - ISIS Godot
        - CraftArms input
            - Bevy Events

#### Behavior trees

Basic BehaviorTrees that are used for micro deciesion making for sequencing actions. As outlined by [Bobby Anguelov](https://takinginitiative.files.wordpress.com/2020/01/behaviortrees_breaking-the-cycle-of-misuse.pdf), small trees who only affect decisions related to the specfic task, we can avoid complex, hard to extend trees everyone complains about. How will this work out in practice, we'll see I suppose.

## devlog

### -Z is forward

It's either that or -X is right.

### Machine EPSILON

A test for the `smallest_positve_equivalent_angle_rad` on the value `TAU + (PI / 2)` doesn't pass unless I use less than or equal to epsilon. As opposed to less than epsilon. Which I think is pretty damn curious.

### Bug: euler angles for nalgebra

It claims to return roll, pitch yaw but it actually returns pitch, yaw, roll. I'm sure of it.

According to the author:

	Euler angles in nalqebra follows the aircraft convention ,i.e., with yaw the rotation about Z (which is the "up" axis usually chosen in this context).
	Roll: X, Pitch: Y, Yaw: Z
	https://discord.com/channels/507548572338880513/507548945912954881/552583968432586753

### ii as TReal / RAY_COUNT as TReal != (ii / RAY_COUNT) as TReal;

Obviously. This bug might have actually cost me months.
