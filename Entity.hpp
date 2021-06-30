#pragma once
#include "raylib.h"


class Entity {
  public:
    Rectangle rectangle;
    Vector2 position; // this is the center of the Entity
    Vector2 velocity;
    Color color;

    Entity(Vector2, int, int);
    Entity(Vector2, int, int, Color);
    Entity(int, int, int, int);

    Vector2 getPosition();
    Vector2 getVelocity();
    Rectangle getRectangle();
    Rectangle* getRectanglePtr();
    void setColor(Color);
    bool checkCollision(Rectangle);
    virtual void update();
    void draw();
  protected:
    void updateRectangle();
    void updatePosition();
};