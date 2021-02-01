Open in app
Get started
Capital One Tech
Responses (4)

To respond to this story,
get the free Medium app.
Open in app
Michele d'Amico
Michele d'Amico
almost 3 years ago

10
3
Great article but why don’t take the ownership in apply() method?
Your trait could be
Noël Widmer
Noël Widmer
over 2 years ago

Love the article and the topic you are writing about. My team and I are working in the exact same domain as you (based on your writing) and are applying DDD and ES in an object oriented environment. It’s great to see a sample of your ideas and to…...
Read More
3
Reply
XiaoPeng
XiaoPeng
10 months ago

That was, until I envisioned the “switch” statement where I would have to switch on type, rather than content. That felt like a warning signal to me that I was on the wrong track.
Isn’t that a benefit from the strong-typing point of view?
Reply
Richard Pringle
Richard Pringle
about 1 year ago

The move semantics here are great. I think this is a completely new way to think about immutability. The whole point of persisted-structures is to prevent the proverbial rug from being pulled out from under you by some other function. However, if…...
Read More
Reply
Event Sourcing with Aggregates in Rust
Kevin Hoffman
Kevin Hoffman
Feb 12, 2018 · 10 min read



Image for post
Every developer loves event sourcing, right up until the moment they have to implement it. At that moment all the wonderful whiteboard drawings and inspirations gleaned from conference attendance come to a screeching halt as you try and map this pattern to your business domain.
Admittedly I’ve been in that sinking boat. I am a huge advocate for immutable streams of data, event sourcing, separation of command and query, and the benefits of treating state as a function of an event stream. However, given all of that, I’ve only been able to implement a handful of “pure” event sourcing systems. Hacks show up — they bleed out of open wounds, they crawl in through chew-holes in your walls like rodents, they wreck everything.
If you’re new to the concept, event sourcing is the idea that your shared state is not mutable in place. It is instead the result of sequentially applying immutable events that represent something that took place in the past. As I say in my books, Cloud Native Go and Microservices with ASP.NET Core, reality is event-sourced. Your brain consumes multiple streams of input and calculates state (our concept of “reality”) based on stored history in the mind, sensory inputs, etc. Once you start envisioning problems as event sourcing problems, it’s very difficult to see them any other way.
The aggregate is a domain-driven-design (DDD) concept that fits well within event sourcing. To put it as briefly as possible: you apply a command to an aggregate which then produces one or more events. An aggregate can populate (re-hydrate) its state by sequential application of an event stream.
For more information on this topic, feel free to go down the rabbit hole of googling event sourcing. For the remainder of this post, I want to discuss how we can implement something like event sourcing and aggregates in Rust without letting the hacks creep in through the cracks.
The first question is — how do you represent an event? In my sample, I’m working with a domain where we have products (uniquely identified by a SKU) that can have a measurable on-hand quantity within a warehouse. Events that can happen to a product are things like being reserved (some quantity is “spoken for” by an un-shipped order), released (an order is canceled), or shipped (previously reserved quantity actually leaves the warehouse).
My first instinct is to create some structs:
ProductShipped
ProductReserved
ProductReleased
Each one of these structs would then have some payload data like timestamp, etc. I can then serialize these onto queues for handling downstream. On the surface, this originally felt like a good idea to me.
That was, until I envisioned the “switch” statement where I would have to switch on type, rather than content. That felt like a warning signal to me that I was on the wrong track. With Rust, I can easily pattern match on content, so instead I decided to use enums with struct variants:
struct ProductEventData {
    quantity: usize,
    timestamp: usize,
    sku: String,
}

enum ProductEvent {
    Reserved(ProductEventData),
    Released(ProductEventData),
    Shipped(ProductEventData),
}
Here I’ve got a struct that I can derive serialization for if I want, but the interesting part is that now I have ProductEvent::* enums that represent each of the possible types of events that can occur to a product, and I can treat all product events as a single type. This comes in really handy when I want to build an aggregate.
As mentioned, an aggregate represents calculated state. However, it is also the thing on which commands are executed. A command is an imperative given to the aggregate for validation. If the command succeeds, then the aggregate will return a list of events ready for emission. Some ES implementations like to have the aggregates emit the events directly, but I like to separate event emission as its own concern, allowing me to test my business logic directly without having to mock a queue client.
As an example, a product aggregate might have a command called ship(…). If this succeeds, I might get the ProductEvent::Shipped(...) enum in response. I could get multiple events in response to a single command which is actually the case in many domains.
So how do we define an aggregate? All aggregates must be able to apply events sequentially to their state. Put psuedo-mathematically, that looks something like this:
f(state`1 + event) = state`2
The result of applying one event to an aggregate produces an aggregate with a new state. This is a very functional way of looking at it and I’ll explain more about my affinity towards immutability later.
Next let’s build a trait that describes an aggregate of events:
trait Aggregate {
    type Item;

    fn version(&self) -> u64;
    fn apply(&self, evt: &Self::Item) -> Self where Self:Sized;
}
Here we’re saying that all aggregates must have a version, and that they must all have an apply method that takes an event of type Item and produces a new thing of type Self (the real type of the thing implementing the trait, not the trait type itself). Requiring Self:Sized just means that we need something with a predictable memory footprint, and not something like a boxed trait object.
Next, because enums are so powerful and flexible in Rust, let’s add some convenience methods so we can create new enums along with their inner struct payloads:
impl ProductEvent {
    fn reserved(sku: &str, qty: usize) -> ProductEvent {
        ProductEvent::Reserved(ProductEventData {
            quantity: qty,
            sku: sku.to_owned(),
            timestamp: 1,
        })
    }
    fn shipped(sku: &str, qty: usize) -> ProductEvent {
        ProductEvent::Shipped(ProductEventData {
            quantity: qty,
            sku: sku.to_owned(),
            timestamp: 1,
        })
    }
    fn released(sku: &str, qty: usize) -> ProductEvent {
        ProductEvent::Released(ProductEventData {
            quantity: qty,
            sku: sku.to_owned(),
            timestamp: 1,
        })
    }
}
Now we’re ready to actually start implementing a domain-specific aggregate, the ProductAggregate, which represents the calculated state of a single product. This is also an important point that I think gets often gets overlooked — aggregates are calculations for a single entity, not for the entire state of your application. Further, aggregates are short-lived. They live long enough to calculate state and validate a command, and that’s it. If you’re building a stateless service, it’s going to dispose of the aggregate at the end of the request.
Here’s our ProductAggregate:
#[derive(Debug)]
struct ProductAggregate {
    version: usize,
    qty_on_hand: usize,
    sku: String,
}
We’re maintaining the version, which is essential for event sourcing implementations of any complexity. This allows us to deal with replay situations (replay to version “x”), and potentially resolve “merge conflicts” when multiple aggregates go to persist at the same time… but that’s a story for another blog post 😀
Let’s put some commands on our aggregate:
impl ProductAggregate {
    fn new(sku: &str, initial_quantity: usize) -> ProductAggregate {
        ProductAggregate {
            version: 1,
            qty_on_hand: initial_quantity,
            sku: sku.to_owned(),
        }
    }

    fn reserve_quantity(&self, qty: usize) -> Result<Vec<ProductEvent>, String> {
        if qty > self.qty_on_hand {
            let msg = format!(
                "Cannot reserve more than on hand quantity ({})",
                self.qty_on_hand
            );
            Err(msg)
        } else if self.version == 0 {
            Err(
                "Cannot apply a command to an un-initialized aggregate. Did you forget something?"
                    .to_owned(),
            )
        } else {
            Ok(vec![ProductEvent::reserved(&self.sku, qty)])
        }
    }

    fn release_quantity(&self, qty: usize) -> Result<Vec<ProductEvent>, String> {
        Ok(vec![ProductEvent::released(&self.sku, qty)])
    }

    fn ship_quantity(&self, qty: usize) -> Result<Vec<ProductEvent>, String> {
        Ok(vec![ProductEvent::shipped(&self.sku, qty)])
    }

    fn quantity(&self) -> usize {
        self.qty_on_hand
    }
}
I’m trying to keep the domain logic simple so we can keep our eyes on the important pieces (the command pattern). In the case of reserve_quantity, we’ll return an Err if we attempt to reserve more stock than we have on hand. In a real-world app there would likely be more involved validation steps, but the Result type here works quite nicely — we get a Vec of events upon success, and we get an Err containing a string otherwise. It’s also worth noting that the returned events are not applied to the aggregate. I am very explicit about keeping the command operation side-effect free and adhering to functional principles as much as possible.
If you want to apply the returned events to the aggregate after you call the command, that’s your choice, but at least those reading your code will know explicitly what’s happening and (hopefully) why.
As you may have guessed, the ability for the aggregate to validate incoming commands relies on the fact that it already has computed state. To compute state, we pump an event stream through the apply method. A lot of people like mutable aggregates here, and they call apply repeatedly on the same aggregate.
For personal reasons stemming from a long and storied history of ugly, difficult to diagnose problems in production created by mutable aggregates, I wanted to take a more functional approach and have my apply method return a brand new aggregate, with the computed state:
impl Aggregate for ProductAggregate {
    type Item = ProductEvent;

    fn version(&self) -> u64 {
        self.version
    }

    fn apply(&self, evt: &ProductEvent) -> ProductAggregate {
      ProductAggregate {
        version: self.version + 1,
        qty_on_hand: match evt {
           &ProductEvent::Released(
            ProductEventData { quantity: qty, .. }) => {
                  self.qty_on_hand + qty
           },
           &ProductEvent::Reserved(
            ProductEventData { quantity: qty, .. }) => {
                    self.qty_on_hand - qty
           },
            _ => self.qty_on_hand,
          },
        sku: self.sku.clone(),
       }
   }
}
There’s a ton of really cool Rust happening in the preceding code. First, you’ll notice that I’m using an associated type to indicate that the type of Items this aggregate processes are ProductEvent variants. I can then use destructuring in my pattern match to pull out just the qty field of the ProductEventData struct in order to produce the newly calculated aggregate.
It was pointed out to me that there’s an even better way to do the apply. In my paranoid revolt against mutability, I overlooked Rust’s concept of the move. In Rust, when you assign something, the default behavior is to move the value from one place to the other (leaving the previous location unusable as a variable). During this move, it’s safe (and often preferred) to do mutations because you’re guaranteed that no other code is referring to the thing you’re mutating at that moment. This is enforced at compile-time and is a simultaneously frustrating and liberating feature.
Embracing the move and mutating the aggregate inside the apply avoids an extra allocation on the stack and avoids the call to clone() on the SKU string:
fn apply(mut self, evt: &ProductEvent) -> ProductAggregate {
  self.version += 1;
  self.qty_on_hand = match evt {
    &ProductEvent::Released(
       ProductEventData{quantity:qty,..}) => 
         self.qty_on_hand.checked_add(qty).unwrap_or(0),
    &ProductEvent::Reserved(
       ProductEventData{quantity:qty,..}) => 
         self.qty_on_hand.checked_sub(qty).unwrap_or(0),
    _ => self.qty_on_hand,
  };
  self
}
This alternate version is more efficient, easier to read, and it also highlights a subtle but critical point: there are calls to checked_sub or checked_addthat convert a potentially panic-inducing math operation into an Option type. Here I’m calling unwrap_or(0) to use the value 0 if the real quantity would have produced an overflow.
This lets me bring up another area of event sourcing that we often neglect: corruption of aggregates. What should really happen here is we should set self.over_reserved (or some similar indicator of trouble) to true ifchecked_sub returned None. If we’re being really diligent, we might also want to save a reference to the event that produced the corruption.
Ideally, nothing should have made it into the persistent event store that would have corrupted the aggregate, but we should always be on guard for things like this. A bad piece of code in a patch to the command validator might have let through an event that can mess up our aggregates, especially if the command and query services are isolated. This kind of logic can produce cascading failures. If your code makes a decision based on silently suppressing a bad event sequence for an aggregate, it can create further corrupt events downstream. Never let a bad event produce a good aggregate.
Finally, now that we’ve implemented both the command and the apply aspects of an aggregate, we can call apply to compute state:
let soap = ProductAggregate::new("SOAP", 100);
let u = soap.apply(&ProductEvent::reserved("SOAP", 10));
let u = u.apply(&ProductEvent::reserved("SOAP", 30));
Another subtle point here is that the apply method takes a reference to the event. This explicitly tells the consumer of the aggregate that it does not claim ownership of the event. If we are using the more efficient version of apply, then if we attempt to refer to soap after the first call to apply, we would run afoul of Rust’s infamous borrow-checker (the error message would show the exact line producing the failure and say something like “value used here after move”).
The preceding code is okay, but since we’re using Rust I think we can do better. Earlier I mentioned that there may have been ulterior motives to wanting a more functional approach to state calculation. Those who have been doing work in functional languages for a while may recognize that the repeated re-assignment of a thing to a new version of the thing is exactly what a fold does.
We can convert a vector of events into an iterator, and pipe them through a fold like so:
let agg = v.iter()
    .fold(ProductAggregate::new("CEREAL", 100), |a, ref b| {
        a.apply(b)
    });
We could write it even more simply if we already have the initial product:
let agg = v.iter().fold(init, |a, ref b| a.apply(b));
Now this is where things start to pay off. There’s a lot of power happening in there. In Rust, mutability is chosen by the consumer, not dictated by the struct. So the init parameter to the fold, my “first” value, is an immutable aggregate with an initial on-hand quantity of 100. I can then call fold and in each of the iterations of the fold, I simply return a.apply(b). In the more efficient version of apply, this moves the aggregate into the return value instead of allocating a new one. Now we start to see some benefits to that refactor: the bigger the vector of events, the worse the “copy and allocate” version of apply would have performed.
I am not passing a mutable value throughout the fold. Instead, I am returning a new, immutable value after each step. At the end, the agg variable is immutable and will have all of my calculated state. Because I’ve made all my command methods return events rather than mutate the aggregate itself, I can then issue a command directly on the agg variable without having to worry about changing mutability or conflicting with Rust’s borrow checker.
In conclusion, this is just one event sourcing practitioner’s opinion and sample implementation. My goal here wasn’t to present you all with the best-ever ES implementation, but to remind you that we can still have all the low-level performance and safety of Rust while still producing simple and elegant code for high-level patterns like event sourcing and CQRS.
DISCLOSURE STATEMENT: These opinions are those of the author. Unless noted otherwise in this post, Capital One is not affiliated with, nor is it endorsed by, any of the companies mentioned. All trademarks and other intellectual property used or displayed are the ownership of their respective owners. This article is © 2018 Capital One.
Sign up for Capital One Tech
By Capital One Tech
The low down on our high tech from the engineering experts at Capital One. Learn about the solutions, ideas and stories driving our tech transformation. Take a look

Your email
Get this newsletter
By signing up, you will create a Medium account if you don’t already have one. Review our Privacy Policy for more information about our privacy practices.
Rust
Event Sourcing
Aggregation
Cqrs
Reactive
864

4



Kevin Hoffman
WRITTEN BY

Kevin Hoffman
Follow
In relentless pursuit of elegant simplicity. Tinkerer, writer of tech, fantasy, and sci-fi. Converting napkin drawings into code for @CapitalOne
Capital One Tech
Capital One Tech

Follow
The low down on our high tech from the engineering experts at Capital One. Learn about the solutions, ideas and stories driving our tech transformation.
More From Medium
Python Flask API — Starter Kit and Project Layout
Juan Cruz Martinez

Understanding Kubernetes Resource (CPU and Memory) Units
Tremaine Eto in The Startup

ASP.NET Core 3 - Authorization and Authentication with Bearer and JWT
Marcos Vinicios in The Innovation

How To Squash Bugs Using Git Bisect
Alexis Määttä Vinkler in Better Programming

Why Your App Shouldn’t Use HTTP. Anywhere.
Josh Cummings in The Startup

An introduction on Pipenv
Arastoo

Dockerizing Rails Applications Part 2: Automation
Al-Waleed Shihadeh in Better Programming

Building Amazon EKS Clusters with Graviton2 Instances using Terraform
Niall Thomson

About
Help
Legal
Get the Medium app
A button that says 'Download on the App Store', and if clicked it will lead you to the iOS App store
A button that says 'Get it on, Google Play', and if clicked it will lead you to the Google Play store
To make Medium work, we log user data. By using Medium, you agree to our Privacy Policy, including cookie policy.



