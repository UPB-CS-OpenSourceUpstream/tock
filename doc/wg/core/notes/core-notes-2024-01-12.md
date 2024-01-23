# Tock Meeting Notes 01/12/24 ===================================

## Attendees
- Hudson Ayers
- Amit Levy
- Leon Schuermann
- Branden Ghena
- Alyssa Haroldsen
- Jonathan Van Why
- Andrew Imwalle
- Tyler Potyondy
- Alexandru
- Brad Campbell
- Philip Levis


## Updates
- Phil: Could we start sending out a link to the new dialpad in the weekly email?
- Hudson: Yes, sure.
- Alexandru: Can we also have a calendar invite for this meeting?
- Amit: Yeah sure thing
- Alexandru: That would be helpful for the time difference with daylight savings
- Alexandru: I can do networking working group updates. We discussed yesterday about finalizing the packet buffer and we are hopefully on the right track with Leon's working prototype. It seems we need to define an API for it, and the consensus is we will define the API as we go. I have a student who is going to build a smarter console for Tock, a desktop application that will show outputs from different apps in different windows using Leon's packet buffer. I think Leon and Tyler are really close to getting Thread working on Tock, and they will use a C app to insert Thread stack into Tock. The target for testing packet buffer is TockWorld with a working proof of concept.
- Branden: Networking working group meeting are moving to Mondays at 11am Eastern, anyone is welcome to join
- Brad: This console project is one of those that hasnt worked yet
- Alex: I am hopefuly the packet buffer will help. The alternative console driver will just add headers to messages and then the desktop app will parse those. Eventually, the goal would be to merge this with the current console in a transparent way, where the headers are only added if the desktop app is detected.
- Brad: Yeah, this would be really great.
- Alex: Plan would be to integrate this into tockloader-rs.
- Leon: I have an update on the PMP design and implementation. The consensus has been we just need to do the work to get CI passing and test this on some proper boards. We had some issues where QEMU had a broken ePMP implementation, then the PMP implementation in the liteX RISC-V boards was broken, I fixed that as well. The PR is ready, and this is a major change to a core subsystem, 3.5k line PR, and we really want to get this right given how much we have iterated on this interface. I would really appreciate people taking a final look at this, proposing any changes needed, and ultimately merging it.

## PR Roundup
- Hudson: Brad shared an update Rust PR, I see that is in the merge queue now, which is exciting.
- Hudson: Brad also shared two Component Type PRs -- PR 3774 and 3775, I assume you were asking people to quickly review these? I will take a look at those.
- Hudson: Brad shared a TicKV fix, PR 3776, I assume this was also a review request. I see you already got Alistairs approval. 
- Hudson: Brad also shared a link to his precompiled library mechanism for libtock-c. Do you want to talk about that?
- Brad: I do not
- Branden: It has three approvals, does anyone have any last concerns?
- Amit: What are we waiting on for this?
- Brad: Nothing on my end
- Amit: It looks like it needs to be rebased or something
- Brad: It does?
- Amit: I can do an update with rebase
- Brad: I can't imagine much has changed
- Amit: Ok, I am hitting merge when ready on this
- Hudson: Brad also has a collection of documentation PRs. There is PR 3777 which is an update to the syscall docs. There is 3778 which removes courses from the Tock repo. I assume the idea here is these courses are out of date and possibly misleading and get in the way when you grep for stuff?
- Brad: Yeah I think we forgot those were there.
- Phil: I think we should not delete those but clearly mark them as old.
- Phil: Nobody will look back through the git history for these, I don't like deleting the history of educational materials.
- Brad: I agree on some level, but I think we are unlikley to resuscitate these, and we have so much better material now.
- Phil: This is something that is coming up at Stanford for old courses too. This is important as educational materials and to showcase history.
- Phil: What if someone wants to find a course they took in 2018?
- Branden: Would a seperate repo to archive these be appropriate?
- Phil: Sure
- Phil: I can be the person to make a separate repo
- Hudson: I assume we could just put them in the Tock-archive repo
- Phil: OK, great, I can do that since I am being the squeaky wheel.
- Hudson: Next up, Brad has been moving technical docs from the Tock repo to the Tock book. There is PR 3779 to the Tock repo to remove them from the Tock repo, and then tock-book repo PR #23 to move them into the book. I know a lot of people do not have notifications enabled for that repo so wanted to call that out.
- Brad: Yeah -- we have this book that is searchable, has a nice ToC, other instructional information, and we have things in various places. I would like to see the de-facto place for documentation about Tock and its implementation be in one place, and that one place be consistent and have the nice trappings of mdbook. I like the automatic table of contents. But there is some documentation about the project, or about how we manage Tock, that does not make sense to move in my opinion because it more so documents the Tock repository, so I think it should stay.
- Hudson: It does seem the main downside here is people do not neccessarily find the book as easily as they find the doc folder.
- Amit: An additional downside is that the goal of the book should be to educate/teach, and the goal of documentation is primarily to serve as a more formal reference. So I don't think that things like TRDs should move out of the Tock repo -- and I realize Brads PR does not do that. And there are probably other kinds of documentation that do not belong in the book.
- Amit: I think only having material that will eventually be phrased/formatted for teaching would lose a more complete/formal reference. Maybe that stuff would be better as doc comments or TRDs.
- Brad: Two things: some stuff would not move to the book (TRDs)...
- Brad: To me, all of this stuff is teaching -- it tells you why a design is the way it is or how different pieces work like TBF headers.
- Amit: I was not disagreeing with anything you moved, but expanding on the set of things that might not belong in the book beyond what you mentioned.
- Brad: Got it.
- Leon: I like that currently PRs can atomically update an implementation and its documentation
- Leon: I do not know if that is a problem that needs to be solved, but this could be bad for documentation that really related to the peculiarities of a particular subsystem.
- Brad: I think that is one of the main reasons we have not done this. However having read our documentation this is already not happening. I had to make a few correctness fixes during this process already. Also, most of our documentation PRs are just doc PRs. there is not much overlap with development PRs. There is not much in our docs that is that implementation specific.
- Amit: I agree with that. In practice there is not much there to be lost.
- Branden: My biggest concern is just the discoverability aspect. I do not want someone to not find the docs. I agree that could be handled with links in the main repo.
- Brad: This is not something I had thought of -- when I look for rust documentation I do not go to their github! I just search, and end up with one of their three different books.
- Branden: But think about how you look up Contiki documentation
- Phil: Both are true, some people do it both ways and you want to accomodate both.
- Amit: Can your doc removal update the main README at the top to add a link to the Tock book?
- Brad: Sure
- Amit: Or a documentation heading somewhere prominent.
- Brad: I should have done that anyway.
- Amit: And maybe we should be doing some other discoverability things, like the tockos.ors website has the Tock book second to last, and has links to documentation in the website repo itself that I would bet are out of date.
- Brad: Yeah I agree, I do not really have a vision for what the tockos website is supposed to do.
- Amit: Haha, I spent most of my time building it figuring how to get the Tock SVG to tick.
- Tyler: I think the Tock book was one of the more helpful resources when I was getting involved, and I only found it through the website, adding it to the README would be very helpful.
- Brad: Yeah, I want us to always send people one place, which will eventually make things much more intuitive than they are now.
- Hudson: Brad had two other PRs -- you updated the HOTP tutorial.
- Brad: Yeah, we switched from app state to key value in the source code and this completes that update, it is also related to those component type PRs, the whole point of that is to make writing tutorials like this easier so they are not tied to one specific hardware platform.
- Hudson: And last was book PR #29, which added TRDs to the book. Are those duplicated?
- Brad: Those are just links to where the source is actually stored.
- Hudson: And they just show up as though they are inline in the book?
- Brad: That is right.
- Amit: Because this is pointing to master, I believe this is going to pull in text at compile time, when the book is compiled, so there is a bit of weirdness of rebuilding the same version of the book gives a different result, and if it does not get rebuilt for a while it might be out of date. But I see the value.
- Hudson: What is the cadence on which the online version of the book is updated if people are not submitting PRs to it?
- Amit: Never
- Hudson: Could we configure netlify to do a periodic rebuild?
- Amit: Probably, but we have not.
- Branden: Or trigger tock-book's netlify from the Tock repo?
- Amit: We could, but...
- Hudson: Yeah, a medium amount of work. Regardless. I am pro-this design.
- Tyler: I wanna circle back to the HOTP tutorial. I have some undergrads I gave that tutorial and some parts were not working. IIRC I also replicated the issue. On your end is it still all working?
- Brad: I have tested it a little bit but not every step. The problem is that the end-to-end thing almost worked and it has been slow to re-update it. It just needs a bit of love when we are not stressed and rushing at the last minute. We really need to get this version to something that does not require committing a custom board to the repo.
- Brad: There is another libtock-c PR I did not put in the email on this topic.
- Brad: As far as the TRDs go, in theory they are not changing, and the ones that are in the works could just not be included in the book, I don't think these things change that much.
- Branden: I agree with that Brad.
- Branden: Any last agenda items?

## OT Calls
- Johnathan: Have the OT working group calls moved?
- Brad: They have not been occurring, it has been pretty much just down to the two of us.
- Johnathan: OK, wanted to make sure I was not unintentionally no-showing.
- Johnathan: Leon, I just saw your comments on Alistairs PR. There is still something wrong. Alistairs PR only fixes one out of the eight pins, but I am gonna go through and fix that whole config.
- Leon: That is great to hear. The way I have been testing this is compiling and bootstrapping CW310 and seeing if it prints.
- Leon: I would have liked to see which commit actually broke this.
- Johnathan: It is from when Michał redid the pinmux driver.
- Leon: More generally, I am now in a position where using the test infra I introduced last week I can run things on a CW310 and it works, which will be pretty nice.
- Johnathan: I am about to go through a transition, since we are moving to hyberdebug with a different board.
- Leon: Did you get a CW340?
- Johnathan: No, a different board. I do not have it yet but will soon.

## Tock World 7
- Brad: What do we need to do for Tock World 7 to be able to advertise it?
- Amit: We need to decide on ticket pricing, high level schedule, maybe venues
- Brad: And what is our current ticket pricing?
- Amit: Whatever it was that you disagreed with
- Andrew: It was $100 on the website
- Phil: Where does that number come from?
- Amit: I pulled it out of thin air when setting up the eventbrite.
- Phil: But what are our costs? For instance having academic resources bring our costs down.
- Amit: Yes. We do not know our costs, in practice, we also have funding we could use to run it.
- Phil: I suggest set the price based on things that scale with people, like food, and for fixed cost things, assume we have funding for that.
- Amit: Yes
- Brad: So how much is that?
- Phil: Depends on location
- Amit: I think we need Pat to figure that out with UCSD people.
- Branden: Brad or I can ballpark from when we each hosted.
- Phil: I would add 50% for UCSD.
- Phil: For tinyOS tech exchanges the host would often cover food because it was small.
- Amit: Yeah, that is what we have done so far.
- Tyler: Did we settle on June 26-28?
- Amit: Yes
- Brad: I get it, but we are just pushing things back. If we put it in the critical path we are just delaying this and then the next things is its April.
- Amit: No it is not in the critical path. We are gonna do cost of UVA food + 50%. And then if we are wrong it is ok because we have funding.
- Phil: And if it is an overestimate give slightly nicer food.
- Amit: We were discussing a hybrid schedule, I put something on the site that said one day for ecosystem stuff, one day for tutorials, and third day as a core-dev day. That mattered because you and Pat suggested putting the tutorial at the beginning or end because it might want a different venue.
- Amit: I do not think we want to change those dates after the fact.
- Amit: Lets put tutorials on the Friday?
- Brad: When Pat and I talked about it we said ascending levels of openness - core dev, then Tockworld, then tutorial.
- Amit: That works for me. I will update that on the website. Brad and Branden, look at food costs and share that? Then we can advertise.
- Brad: Cool, and then for the day that is open to all, do we have thoughts on speakers / invites?
- Amit: I think we should maybe discuss this a bit offline.