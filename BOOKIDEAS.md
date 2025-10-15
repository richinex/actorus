This chapter covers
Connecting tools to data sources
Composing multi-agent systems using router and supervisor patterns
Debugging, testing, and tracing multi-agent interactions
In chapter 11, we explored the foundations of building AI agents by creating a travel
information agent capable of answering user queries about destinations, routes, and
transportation options. While a single, specialized agent can be powerful, real-world
applications often require the coordination of multiple agents, each handling a distinct area
of expertise. In this chapter, we’ll embark on that journey—transforming our travel
information agent into a robust, multi-agent travel assistant system.
Imagine planning a trip where you not only need up-to-date travel information but also
want to seamlessly book your accommodation. Our enhanced multi-agent travel assistant
will do just that: it will be able to answer travel questions and help you reserve hotels or
bed and breakfasts in your chosen destination. To achieve this, we’ll begin by building a
new agent—the accommodation booking agent.
The accommodation booking agent will empower users to book lodgings from two
different sources. First, it will interface with a local accommodation database, which mainly
features hotel deals and is exposed via a dedicated tool. Second, it will connect to an
external B&B REST API, providing access to a wider selection of bed and breakfast options,
also accessible through its own tool. Depending on user requests, the agent will use one or
both of these tools to deliver relevant accommodation options.
350
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Once we have our new agent in place, we’ll combine it with the travel information agent
from the previous chapter. The result will be a unified, multi-agent travel assistant capable
of fielding a wide variety of travel-related queries, handling both information requests and
accommodation bookings, and even combining both for a more streamlined experience.
Let’s begin by constructing our new accommodation booking agent.
12.1 Building an Accommodation Booking Agent
To build a practical, helpful travel assistant, we need more than just information retrieval—
we need the ability to act. In this section, we’ll develop an accommodation booking agent
from the ground up, starting by building the tools it needs: one for hotel bookings based on
a local room availability database, and another for B&B bookings from an external REST
API. By the end of this section, you’ll have a ReAct-style agent that can check and book
both hotel and B&B rooms in Cornwall.
12.1.1 Hotel Booking Tool
Let’s start by creating the hotel booking tool. To enable our agent to retrieve hotel offers
and availability, we’ll use the LangChain SQL Database Toolkit, which exposes a SQL
database as a set of agent tools. This toolkit makes it straightforward for an agent to run
queries, retrieve hotel details, and check room availability—all through tool calls, without
needing to write raw SQL in your prompts.
The hotel data, including current offers and availability, is stored in a local SQLite
database cornwall_hotels.db which is kept up-to-date by our backend partners. We don’t
need to worry about how the data is pushed—just trust that it’s there and refreshed as
needed.
First, copy the latest script, main_03_01.py, to a new script, main_04_01.py. Then,
prepare your environment:
1. Create a folder named hotel_db.
2. Place the provided SQL schema file cornwall_hotels_schema.sql into
that folder.
3. Open a terminal (inside VS Code or standalone), navigate to the folder,
and create the database with (I have omitted the root of the ch11 folder
for convenience):
Now, let’s check that the database is working. Open the SQLite shell:
Within the SQLite shell, run these queries to verify your setup:
\ch11>cd hotel_db
\ch11>sqlite3 cornwall_hotels.db < cornwall_hotels_schema.sql
\ch11>sqlite3 cornwall_hotels.db
351
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
With the database ready, let’s move on to the Python implementation.
Import the necessary LangChain SQL integration libraries:
Instantiate the SQLite database:
Now, create an instance of the SQL Database toolkit:
That’s it! Now you can access the toolkit’s tools with:
12.1.2 B&B Booking Tool
Next, let’s create a Bed & Breakfast booking tool. This tool will retrieve B&B room
availability from a REST service. For development and testing, we’ll mock this service.
First, we’ll define the return type for our tool, and then create a mock implementation of
the BnB booking service, as shown in Listing 12.1. (For convenience, the example here
uses a reduced set of mock data. You can find the complete implementation in the code
files provided with this book.).
sqlite> .tables
sqlite> SELECT * FROM hotels;
sqlite> SELECT * FROM hotel_room_offers;
from langchain_community.utilities.sql_database import SQLDatabase
from langchain_community.agent_toolkits import SQLDatabaseToolkit
hotel_db = SQLDatabase.from_uri("sqlite:///hotel_db/cornwall_hotels.db")
hotel_db_toolkit = SQLDatabaseToolkit(db=hotel_db, llm=llm_model)
hotel_db_toolkit_tools = hotel_db_toolkit.get_tools()
352
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Now we can define the check_bnb_availability tool:
Listing 12.1 BnB Booking Service
#A Define the BnB availability tool
#B Call the BnB booking service to get the offers
#C Mocked BnB offers
class BnBBookingService: #A
@staticmethod
def get_offers_near_town(town: str, num_rooms: int) -> List[BnBOffer]: #B
# Mocked REST API response: multiple BnBs per destination
mock_bnb_offers = [ #C
# Newquay
{"bnb_id": 1, "bnb_name": "Seaside BnB", "town": "Newquay",
"available_rooms": 3, "price_per_room": 80.0},
{"bnb_id": 2, "bnb_name": "Surfside Guesthouse", "town": "Newquay",
"available_rooms": 2, "price_per_room": 85.0},
# Falmouth
{"bnb_id": 3, "bnb_name": "Harbour View BnB", "town": "Falmouth",
"available_rooms": 4, "price_per_room": 78.0},
{"bnb_id": 4, "bnb_name": "Seafarer's Rest", "town": "Falmouth",
"available_rooms": 1, "price_per_room": 90.0},
# St Austell
{"bnb_id": 5, "bnb_name": "Garden Gate BnB", "town": "St Austell",
"available_rooms": 2, "price_per_room": 82.0},
{"bnb_id": 6, "bnb_name": "Coastal Cottage BnB", "town": "St
Austell", "available_rooms": 3, "price_per_room": 88.0},
...
# Port Isaac
{"bnb_id": 27, "bnb_name": "Port Isaac View BnB", "town": "Port
Isaac", "available_rooms": 2, "price_per_room": 99.0},
{"bnb_id": 28, "bnb_name": "Fisherman's Cottage BnB", "town": "Port
Isaac", "available_rooms": 2, "price_per_room": 101.0},
# Fowey
{"bnb_id": 29, "bnb_name": "Fowey Quay BnB", "town": "Fowey",
"available_rooms": 2, "price_per_room": 94.0},
{"bnb_id": 30, "bnb_name": "Riverside Rest BnB", "town": "Fowey",
"available_rooms": 2, "price_per_room": 96.0},
]
offers = [offer for offer in mock_bnb_offers if offer["town"].lower() ==
town.lower() and offer["available_rooms"] >= num_rooms]
return offers
353
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
12.1.3 ReAct Accommodation Booking Agent
With both the hotel and B&B booking tools ready, it’s time to build the ReAct
accommodation booking agent. This agent will use both tools in response to user requests.
If the user doesn’t specify a preference, the agent will search both hotels and B&Bs for
available rooms.
You can now try out the agent by replacing the agent line in chat_loop() with:
Let’s run the main_04_01.py script in debug mode and ask the following question:
Listing 12.2 BnB Availability tool
#A Define the BnB availability tool
#B Define the input and return type of the BnB availability tool
@tool(description="Check BnB room availability and price for a destination in
Cornwall.") #A
def check_bnb_availability(destination: str, num_rooms: int) -> List[Dict]: #B
"""Check BnB room availability and price for the requested destination and
number of rooms."""
offers = BnBBookingService.get_offers_near_town(destination, num_rooms)
if not offers:
return [{"error": f"No available BnBs found in {destination} for
{num_rooms} rooms."}]
return offers
#A Define the booking tools, which are the tools from the hotel database toolkit and the BnB availability tool
#B Create the accommodation booking agent
BOOKING_TOOLS = hotel_db_toolkit_tools + [check_bnb_availability] #A
accommodation_booking_agent = create_react_agent( #B
model=llm_model,
tools=BOOKING_TOOLS,
state_schema=AgentState,
prompt="You are a helpful assistant that can check hotel and BnB room
availability and price for a destination in Cornwall. You can use the tools to
get the information you need. If the users does not specify the accommodation
type, you should check both hotels and BnBs.",
)
...
result = accommodation_booking_agent.invoke(state)
...
354
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
After you hit ENTER, you might see an answer similar to the following one:
As you can see, the agent used both tools to retrieve results from both the hotel database
and the mock B&B service.
At this point, your accommodation booking agent is working as expected. It’s strongly
recommended to debug the execution and inspect the LangSmith traces to better
understand how the agent is reasoning and acting step by step.
Although you now have both a travel information agent and an accommodation booking
agent, they are still disconnected. You can use either one or the other, but not both in a
unified experience. In the next section, we’ll build a multi-agent travel assistant that brings
these capabilities together, providing a seamless experience for travel planning and
accommodation booking.
12.2 Building a router-based Travel assistant
So far, we have developed two independent agents: a travel information agent and an
accommodation booking agent, each with specialized capabilities. While this modular
approach is powerful, it raises an essential design question: how can we combine these
agents to deliver a seamless user experience—one that can answer travel information
queries and handle accommodation bookings in a single conversation?
A common and effective solution is to introduce a router agent. This agent acts as an
intelligent entry point: it receives the user’s message, determines which specialized agent
should handle the request, and dispatches the task accordingly.
12.2.1 Designing the Router Agent
To implement our multi-agent travel assistant, begin by copying your previous script,
main_04_01.py, to main_05_01.py. Next, we need to bring in some extra libraries to
support graph-based workflows:
UK Travel Assistant (type 'exit' to quit)
You: Are there any rooms available in Penzance?
I have found Penzance Pier BnB with available rooms at £95 per room, and Cornish
Charm BnB with 3 available rooms at £87 per room.
For hotels, Penzance Palace has 3 available rooms with prices of £130 for a
single room and £200 for a double room. Would you like to book a room or need
more information?
from langgraph.graph import StateGraph, END
from langgraph.types import Command
355
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
The next step is to clearly define the set of agents available for routing. We do this by
declaring an enumeration for the two agent types:
To ensure our router agent receives clear and structured decisions from the LLM, we define
a Pydantic model that captures the LLM’s output—specifying which agent should handle
each query:
By configuring the OpenAI LLM client to produce responses in this structured format, we
eliminate any need for string parsing or manual post-processing. The router will always
produce a result of either "travel_info_agent" or "accommodation_booking_agent".
12.2.2 Routing Logic
The heart of the router agent is its system prompt, which concisely instructs the LLM how
to classify each user request:
With this system prompt, the router agent evaluates each user input and decides which
specialist agent should take over. The router is implemented as the entry node of our
LangGraph workflow, with the travel information agent and the accommodation booking
agent as subsequent nodes. You can see the router implementation in listing 12.2.
class AgentType(str, Enum):
travel_info_agent = "travel_info_agent"
accommodation_booking_agent = "accommodation_booking_agent"
class AgentTypeOutput(BaseModel):
agent: AgentType = Field(..., description="Which agent should handle the
query?")
ROUTER_SYSTEM_PROMPT = (
"You are a router. Given the following user message, decide if it is a travel
information question (about destinations, attractions, or general travel info) "
"or an accommodation booking question (about hotels, BnBs, room availability,
or prices).\n"
"If it is a travel information question, respond with 'travel_info_agent'.\n"
"If it is an accommodation booking question, respond with
'accommodation_booking_agent'."
)
356
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
If you examine the implementation in listing 12.2, you’ll notice that the router extracts the
user’s message and submits it to the LLM along with the system prompt. The LLM returns a
structured output of type AgentTypeOutput, which contains the agent name to which the
request should be routed. The router then uses a Command to redirect the conversation flow
to the selected agent node in the graph. In simple workflows, the Command can hand off the
unchanged state to the new node, but it also allows for state updates in more complex
flows.
12.2.3 Building the Multi-Agent Graph
At this point, you have all the components needed to assemble the graph-based multiagent system. You can see the graph implementation in listing 12.3.
Listing 12.3 Router Agent node
#A Get the messages from the state
#B Get the last message from the messages list
#C Check if the last message is a HumanMessage
#D Get the content of the last message
#E Create the router messages, including the system prompt and the user input
#F Invoke the router model, which returns the relevant agent name
#G Get the agent name from the router response
#H Return the command to update the state and go to the agent
#I If the last message is not a HumanMessage, return the command to update the state and go to the
travel_info_agent (default agent)
def router_agent_node(state: AgentState) -> Command[AgentType]:
"""Router node: decides which agent should handle the user query."""
messages = state["messages"] #A
last_msg = messages[-1] if messages else None #B
if isinstance(last_msg, HumanMessage): #C
user_input = last_msg.content #D
router_messages = [ #E
SystemMessage(content=ROUTER_SYSTEM_PROMPT),
HumanMessage(content=user_input)
]
router_response = llm_router.invoke(router_messages) #F
agent_name = router_response.agent.value #G
return Command(update=state, goto=agent_name) #H
return Command(update=state, goto=AgentType.travel_info_agent) #I
357
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
The workflow graph connects the router agent to the two specialized agents. Notably, the
only explicit edges you define are those from the travel information agent and the
accommodation booking agent to the end of the workflow. The connection from the router
to the specialized agents is determined dynamically at runtime by the LLM’s response and is
handled via Command.
Below is a graphical representation of the current multi-agent travel assistant graph:
Listing 12.4 Router-based multi-agent travel assistant graph
#A Define the graph
#B Add the router agent node
#C Add the travel info agent node
#D Add the accommodation booking agent node
#E Add the edge from the travel info agent to the end
#F Add the edge from the accommodation booking agent to the end
#G Set the entry point to the router agent
#H Compile the graph
graph = StateGraph(AgentState) #A
graph.add_node("router_agent", router_agent_node) #B
graph.add_node("travel_info_agent", travel_info_agent) #C
graph.add_node("accommodation_booking_agent", accommodation_booking_agent) #D
graph.add_edge("travel_info_agent", END) #E
graph.add_edge("accommodation_booking_agent", END) #F
graph.set_entry_point("router_agent") #G
travel_assistant = graph.compile() #H
358
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Figure 12.1 Router-based multi-agent Travel assistant: the router agent dispatches user queries to either the
travel information agent or the accommodation booking agent, each equipped with their own specialized tools
As the diagram in figure 12.1 shows, this is a hybrid architecture. At the top, the system
exhibits a deterministic, workflow-driven routing logic. At the lower level, each specialist
agent uses its own set of tools (such as travel data APIs or accommodation booking
interfaces) and follows an agentic, tool-based decision process, which is inherently more
flexible and dynamic.
12.2.4 Trying Out the Router Agent
To see the system in action, run your multi-agent travel assistant by starting
main_05_01.py in debug mode and try the following two user queries:
What are the main attractions in St Ives?
Are there any rooms available in Penzance this weekend?
One important thing to note with this design is that each user question is routed to a single
agent for handling—in other words, each query takes a "one-way ticket" through the
workflow. The router makes a clean and unambiguous handoff, and the selected agent
responds directly to the user before the workflow ends.
For example, when you ask:
the request is routed to the travel information agent. You can see the related LangSmith
execution trace in figure 12.2.
What are the main attractions in St Ives?
359
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Figure 12.2 LangSmith execution trace of a travel information question
If you ask:
Are there any rooms available in Penzance this weekend?
360
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
the system dispatches the query to the accommodation booking agent.
In both cases, the workflow (which can be followed in figure 12.2) is clear: the router
agent evaluates the intent and hands the query off to the most appropriate specialist agent,
which then handles the response and ends the session.
This modular, graph-based design provides a strong foundation for more advanced
workflows. In later sections, you’ll see how you can evolve this system to handle more
complex, multi-step, or even collaborative agentic scenarios.
12.3 Handling Multi-Agent Requests with a Supervisor
The workflow-based multi-agent architecture we developed in the previous section works
well for simple, single-purpose queries—questions that can be clearly routed to either the
travel information agent or the accommodation booking agent. But what happens when a
user asks for something that spans both domains? Consider a query like:
"Can you find a nice seaside Cornwall town with good weather right now and find
availability and price for one double hotel room in that town?"
With our previous router-based design, such a question cannot be answered effectively,
as it requires both agents to work together and share intermediate results.
To solve this, we need to shift our architecture towards a more flexible, collaborative
agent system—one where multiple specialized agents can be coordinated as “sub-tools”
under a higher-level manager. In LangGraph, this is exactly the use case for the
Supervisor: a built-in component designed to orchestrate multiple agents, allowing them to
collaborate on complex requests.
12.3.1 The Supervisor Pattern: An Agent of Agents
Conceptually, the Supervisor is an “agent of agents.” It acts as an orchestrator, managing a
collection of other agents (which themselves may use tools) and deciding which agent to
activate, possibly multiple times in a single workflow. Each agent acts as a specialized tool
that the Supervisor can invoke as needed.
Let’s see how to set up this pattern in your multi-agent travel assistant.
Start by copying one of your previous implementation, main_04_01.py, to a new script,
main_06_01.py. Next, install the necessary package:
Then import the Supervisor:
When defining agents to be managed by the Supervisor, it’s important to assign each a
unique name. You can see how you instantiate your agents with names in listing 12.4.
pip install langgraph-supervisor
from langgraph_supervisor.supervisor import create_supervisor
361
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Now you can implement your travel assistant as a Supervisor, as shown in listing 12.5:
Listing 12.5 Setting up the leaf agents
travel_info_agent = create_react_agent(
model=llm_model,
tools=TOOLS,
state_schema=AgentState,
name="travel_info_agent",
prompt="You are a helpful assistant that can search travel information and
get the weather forecast. Only use the tools to find the information you need
(including town names).",
)
accommodation_booking_agent = create_react_agent(
model=llm_model,
tools=BOOKING_TOOLS,
state_schema=AgentState,
name="accommodation_booking_agent",
prompt="You are a helpful assistant that can check hotel and BnB room
availability and price for a destination in Cornwall. You can use the tools to
get the information you need. If the users does not specify the accommodation
type, you should check both hotels and BnBs.",
)
362
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
You’ll notice that configuring a Supervisor is much like setting up a ReAct agent, but instead
of passing a list of tools, you provide a list of agents. Since the Supervisor needs to analyze
complex multi-step requests and coordinate several agents, it’s best to use a more powerful
LLM model (like gpt-4.1 or even o3) to maximize accuracy and task decomposition.
TIP Try experimenting with different models for the Supervisor, such as gpt-4.1, o3 or o4-mini,
and compare how well the assistant handles increasingly complex, multi-faceted questions.
As in previous designs, simply update your chat loop to invoke the supervisor-based travel
assistant:
Listing 12.6 Setting up the Supervisor agent
#A Create the supervisor
#B Define the agents to be used by the supervisor
#C Define the LLM model to be used by the supervisor to be a high-grade model like gpt-4.1 or even o3
#D Define the prompt for the supervisor (the system prompt) that will be used to guide the supervisor's
behavior
#E Compile the supervisor, which is a LangGraph graph
travel_assistant = create_supervisor( #A
agents=[travel_info_agent, accommodation_booking_agent], #B
model= ChatOpenAI(model="gpt-4.1", use_responses_api=True), #C
supervisor_name="travel_assistant",
prompt=( #D
"You are a supervisor that manages two agents: a travel information agent
and an accommodation booking agent. "
"You can answer user questions that might require calling both agents
when needed. "
"Decide which agent(s) to use for each user request and coordinate their
responses."
)
).compile() #E
result = travel_assistant.invoke(state)
363
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
12.3.2 From “One-Way” to “Return Ticket” Interactions
Unlike the workflow-based router—where every user question was routed once and only
once to a specific agent (a “one-way ticket”)—the Supervisor enables a much richer
interaction. The Supervisor can invoke each agent as needed, potentially revisiting agents
multiple times (“return tickets”) in a single session to collect, combine, and reason over
intermediate results. This enables the system to handle more sophisticated, open-ended,
and multi-part queries.
Below, you can see a diagram representing this supervisor-based architecture:
Figure 12.3 Router-based multi-agent Travel assistant: the router agent dispatches user queries to either the
travel information agent or the accommodation booking agent, each equipped with their own specialized tools
As shown in the diagram in figure 12.3, both the high-level (supervisor) and low-level
(agent/tool) orchestration follow a tool-based approach, maximizing flexibility and
composability. The Supervisor becomes the central decision-maker, ensuring the right agent
(or sequence of agents) is activated for every complex travel request.
This Supervisor-driven architecture unlocks a new level of multi-agent collaboration,
laying the groundwork for more advanced, open-ended AI travel assistants capable of
addressing real-world, multi-step needs.
12.3.3 Trying out the Supervisor agent
Now, run the travel assistant by starting main_06_01.py in debug mode (with LangSmith
tracing enabled), and try entering a complex, multi-part question such as:
364
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
When you examine the LangSmith trace, you’ll notice a more intricate agent utilization
trajectory, similar to that in figure 12.4, in which the travel_assistant is the supervisor
agent.
UK Travel Assistant (type 'exit' to quit)
You: Can you find a nice seaside Cornwall town with good weather right now and
find availability and price for one double hotel room in that town?
365
© Manning Publications Co. To comment go to liveBook
Licensed to Richard Chukwu <richinex@gmail.com>
Figure 12.4 LangSmith execution trace of a combine travel information and booking question
I have summarized below the key steps from the execution trace below, so you can
understand the flow better (remember the travel_assistant is the supervisor):
travel assistant
