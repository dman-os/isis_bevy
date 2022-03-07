#  IN SPACE IN SPACESHIP DEVDOCS

WORLD TO LOCAL - inverse transformation you idiot!

## TODO

- [ ] Surmount technical hurdles
- [ ] Refine loop

- crates to check out
 - [ ] bevy polyline
 - [ ] big brain
 - [ ] bevy remote dev tools

- Consider using arc and weak references to improve performance

- How are we treating unused steering routines?

## design doc

### What we want

#### Features

- [ ] Newtonian flight model
- [ ] Formations
- [ ] Grapple hooks

#### Prior art

- Mount & Blade
- Void Destrotters

#### Focus areas

- Personal combat
- RTS elements
- Living breating world
- Sandbox

### AI

#### Architecture

If we agree that AI needs three abilities:

- Sensors
- Decision making
- Actuation

...we need to design each in an ECS manner.

We'll also need someform of scheduling to distribute work across frames.

- Layers
    - Sensors
        - Are used for process stimuli 
            - Descision makers should be allowed to make own queries
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
            - Blackboard?
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

#### Minds

- Master
  - scenario orchestration
  - responsiblities
    - assign goals to groups
- Tribe
  - faction orchestration
  - responsiblities
    - assign goals to Flocks
- Flock
  - responsiblities
    - assign goals to Boids
    - formations
- Boid
  - individual orchestration
  - responsiblities
    - engine inputs
    - arms inputs
        - assign targets to turrents 

##### Boid Mind

- Strategies
  - [ ] Attack Persue
  - [ ] Follow Waypoints  
- Steering Behvaiors
  - [x] Seek
  - [x] Intercept
  - [x] Cohesion
  - [x] Separation
  - [x] Alignment
  - [x] Arrive

#### Behavior trees

Basic BehaviorTrees that are used for micro deciesion making for sequencing actions. As outlined by [Bobby Anguelov](https://takinginitiative.files.wordpress.com/2020/01/behaviortrees_breaking-the-cycle-of-misuse.pdf), small trees who only affect decisions related to the specfic task, we can avoid complex, hard to extend trees everyone complains about. How will this work out in practice, we'll see I suppose.

#### Formations

Concerns include:

- Varied mechanisms to wait till units are formed.
- Phantom leaders or some mechanism to prevent motion from depending on leaders too much.
- Formations of formations
- Formation lifecycles:
  - Destryoing formation when constraints fail
  - Formation evolution as crafts join/leave

* <https://github.com/libgdx/gdx-ai/wiki/Formation-Motion>
* <https://www.gamasutra.com/view/feature/3314/coordinated_unit_movement.php?print=1>
* <https://sander.landofsand.com/publications/CIG08Heijden.pdf>


Pieces:
- Pivots
  - Pivot motion
    - Copy a leader boid
  - Multi pivot formation
- Slots
  - Assignment strategy
    - Hard roles


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

### `ii as TReal / RAY_COUNT as TReal != (ii / RAY_COUNT) as TReal`;

Obviously.

### The Overengineering of `ActiveRoutines`

Here's what I want. I want my descision making layer to be able to juggle a number of steering routines, using one in one frame and choosing another in frame two. As I see it, they'll be mostly reusing a few (one, two or three) routines, switching amongst them, I don't want to have to make new Routines everytime the descisions are made. Following?

Steering routines are entities that have a parameter component and systems that batch processes all routines of one kind (in true ECS fashion). All our routines are tagged with the `SteeringRoutine` component. In order to avoid computing work for `SteeringRoutine`s currently not in use, we have an `ActiveRoutine` filter tag for filtering (pre-optimization you dolt!). In essence, the descision maker will tag the routines it wants used with `ActiveRoutine`. Makes sense?

Now, in order to simplfy coding, I broke down the descision making layer into a set of systems/components.

- `RoutineComposer`: composes the output of all the currently `ActiveRoutines` into a single output using some method. (Weighted sum or priority override)
- `Strategy`: this is a yet to be written layer that's supposed to be doing the acutal deciding.

An obvious simplification, as it appeared to me, is instead of having the `Strategy` system tag and detag things, it'll output a `RoutineComposer` component and we'll tag all the currently being composed routines with `ActiveRoutine`. Also, we're mantaining an index of a craft's routines, idekwhy. 

---

I think I miss OOP

---

Composablitiy. I see now that this is driven by a want for composability. I have no idea how this's supposed to help though.

---

I'm 110% sure I over-engineerined every inch of this. Hope it's usable. Also, watch out for everything decomposed to small components/systems.

### Figuring out how to structure the AI

So far, I know that we'll have Boids, i.e. units or single crafts, and the Master, the entity responsible for orchestrating the whole game session for all agents. But there are multitudes of organizational needs beyond the master. To start, we'll have factions in the traditional RTS sense. But then, it starts to get blurry. We'll need some form grouping method to allow the following functions:

- Formations
- Assign common objectives (not necessarily in formation)

These and more I can't think of right now. Also, there should be a fleshed out way to assign objectives without the need for unit groups.

How about we model this along the axis of independence?

- Master: orchestrate gameplay to be fun
- Tribe: orchestrate a single faction
- Flock: A unit or group of units that can recieve orders
  - Must support single boid operations
  - Find a better name
- Boid: a single craft 